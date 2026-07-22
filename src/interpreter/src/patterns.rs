use crate::*;
use std::collections::HashMap;

const RUNTIME_CONTEXT_PATTERN_VALUE_PREFIX: &str = "mech-internal-context-";

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
    pub kind: Option<ValueKind>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatternBinding {
    pub index: usize,
    pub id: u64,
    pub name: String,
    pub kind: ValueKind,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq)]
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
        expected_kind: Option<ValueKind>,
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
        enum_id: Option<u64>,
        variant_id: u64,
        payload: Option<Box<CompiledPattern>>,
    },
    AtomTuple {
        tag_id: u64,
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
                    expected_kind,
                } => {
                    if specs.len() <= *binding_index {
                        specs.resize(*binding_index + 1, None);
                    }
                    match &mut specs[*binding_index] {
                        Some(spec) if spec.kind.is_none() && expected_kind.is_some() => {
                            spec.kind = expected_kind.clone();
                        }
                        Some(_) => {}
                        slot @ None => {
                            *slot = Some(PatternBindingSpec {
                                index: *binding_index,
                                id: *id,
                                name: name.clone(),
                                kind: expected_kind.clone(),
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
        expected_kind: Option<&ValueKind>,
    ) -> MResult<CompiledPattern> {
        let expected_kind = expected_kind.map(ValueKind::deref_kind);
        if let Some(index) = self.binding_ids.get(&id).copied() {
            let existing_kind = self.bindings[index].kind.clone();
            match (&existing_kind, &expected_kind) {
                (Some(existing), Some(expected)) if existing != expected => {
                    return Err(self.error(
                        pattern,
                        format!(
                            "Repeated binding '{}' has incompatible kinds '{}' and '{}'.",
                            name, existing, expected
                        ),
                    ));
                }
                (None, Some(expected)) => self.bindings[index].kind = Some(expected.clone()),
                _ => {}
            }
            return Ok(CompiledPattern::Binding {
                binding_index: index,
                id,
                name,
                expected_kind,
            });
        }

        let index = self.bindings.len();
        self.binding_ids.insert(id, index);
        self.bindings.push(PatternBindingSpec {
            index,
            id,
            name: name.clone(),
            kind: expected_kind.clone(),
        });
        Ok(CompiledPattern::Binding {
            binding_index: index,
            id,
            name,
            expected_kind,
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
        expected_kind: Option<&ValueKind>,
        interpreter: &Interpreter,
    ) -> MResult<CompiledPattern> {
        match pattern {
            Pattern::Wildcard => Ok(CompiledPattern::Wildcard),
            Pattern::Expression(expression) => {
                #[cfg(feature = "enum")]
                if let Expression::Literal(Literal::Atom(atom)) = expression {
                    if let Some(ValueKind::Enum(enum_id, _)) =
                        expected_kind.map(ValueKind::deref_kind)
                    {
                        let variant_id = atom.name.hash();
                        let enum_definition = interpreter
                            .state
                            .borrow()
                            .enums
                            .get(&enum_id)
                            .cloned()
                            .ok_or_else(|| {
                                self.error(
                                    pattern,
                                    format!("Enum kind '{}' has no registered definition.", enum_id),
                                )
                            })?;
                        let declared_payload = enum_definition
                            .variants
                            .iter()
                            .find(|(id, _)| *id == variant_id)
                            .map(|(_, payload)| payload)
                            .ok_or_else(|| {
                                self.error(
                                    pattern,
                                    format!(
                                        "'{}' is not a variant of the expected enum.",
                                        atom.name.to_string()
                                    ),
                                )
                            })?;
                        if declared_payload.is_some() {
                            return Err(self.error(
                                pattern,
                                "Enum variant pattern is missing its payload pattern.",
                            ));
                        }
                        return Ok(CompiledPattern::EnumVariant {
                            enum_id: Some(enum_id),
                            variant_id,
                            payload: None,
                        });
                    }
                }
                if let Some(var) = extract_pattern_variable(expression) {
                    self.compile_binding(
                        pattern,
                        var.name.hash(),
                        var.name.to_string(),
                        expected_kind,
                    )
                } else {
                    Ok(self.compile_expression(expression))
                }
            }
            Pattern::Tuple(tuple) => {
                let expected_elements = match expected_kind.map(ValueKind::deref_kind) {
                    Some(ValueKind::Tuple(elements)) => {
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
                            format!("Tuple pattern cannot match expected kind '{}'.", kind),
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
                let (element_kind, rest_kind) = match expected_kind.map(ValueKind::deref_kind) {
                    Some(ValueKind::Matrix(element, _)) => {
                        let element = *element;
                        (
                            Some(element.clone()),
                            Some(ValueKind::Matrix(Box::new(element), Vec::new())),
                        )
                    }
                    Some(kind) => {
                        return Err(self.error(
                            pattern,
                            format!("Array pattern cannot match expected kind '{}'.", kind),
                        ));
                    }
                    None => (None, None),
                };
                let prefix = array
                    .prefix
                    .iter()
                    .map(|element| self.compile(element, element_kind.as_ref(), interpreter))
                    .collect::<MResult<Vec<_>>>()?;
                let spread = match &array.spread {
                    Some(spread) => Some(CompiledPatternArraySpread {
                        kind: spread.kind.clone(),
                        binding: match &spread.binding {
                            Some(binding) => Some(Box::new(self.compile(
                                binding,
                                rest_kind.as_ref(),
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
                    .map(|element| self.compile(element, element_kind.as_ref(), interpreter))
                    .collect::<MResult<Vec<_>>>()?;
                Ok(CompiledPattern::Array {
                    prefix,
                    spread,
                    suffix,
                })
            }
            Pattern::TupleStruct(tuple_struct) => match expected_kind.map(ValueKind::deref_kind) {
                Some(ValueKind::Enum(enum_id, _)) => {
                    #[cfg(feature = "enum")]
                    {
                        let enum_definition = interpreter
                            .state
                            .borrow()
                            .enums
                            .get(&enum_id)
                            .cloned()
                            .ok_or_else(|| {
                                self.error(
                                    pattern,
                                    format!(
                                        "Enum kind '{}' has no registered definition.",
                                        enum_id
                                    ),
                                )
                            })?;
                        let variant_id = tuple_struct.name.hash();
                        let declared_payload = enum_definition
                            .variants
                            .iter()
                            .find(|(id, _)| *id == variant_id)
                            .map(|(_, payload)| payload.clone())
                            .ok_or_else(|| {
                                self.error(
                                    pattern,
                                    format!(
                                        "'{}' is not a variant of the expected enum.",
                                        tuple_struct.name.to_string()
                                    ),
                                )
                            })?;
                        let payload = match (tuple_struct.patterns.as_slice(), declared_payload) {
                            ([], None) => None,
                            ([payload_pattern], Some(Value::Kind(payload_kind))) => Some(Box::new(
                                self.compile(payload_pattern, Some(&payload_kind), interpreter)?,
                            )),
                            _ => {
                                return Err(self.error(
                    pattern,
                    "Enum variant pattern payload arity does not match its definition.",
                  ));
                            }
                        };
                        return Ok(CompiledPattern::EnumVariant {
                            enum_id: Some(enum_id),
                            variant_id,
                            payload,
                        });
                    }
                    #[cfg(not(feature = "enum"))]
                    {
                        return Err(self.error(pattern, "Enum patterns are not enabled."));
                    }
                }
                Some(ValueKind::Tuple(kinds)) => {
                    if kinds.len() != tuple_struct.patterns.len() + 1
                        || !matches!(kinds.first(), Some(ValueKind::Atom(_, _)))
                    {
                        return Err(self.error(
                pattern,
                "Atom-tagged tuple pattern arity does not match the expected tuple kind.",
              ));
                    }
                    let payload = tuple_struct
                        .patterns
                        .iter()
                        .zip(kinds.iter().skip(1))
                        .map(|(payload, kind)| self.compile(payload, Some(kind), interpreter))
                        .collect::<MResult<Vec<_>>>()?;
                    Ok(CompiledPattern::AtomTuple {
                        tag_id: tuple_struct.name.hash(),
                        payload,
                    })
                }
                Some(kind) => Err(self.error(
                    pattern,
                    format!(
                        "Tagged tuple pattern cannot match expected kind '{}'.",
                        kind
                    ),
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
            tag_id: tuple_struct.name.hash(),
            payload,
        })
    }
}

pub fn compile_pattern(
    pattern: &Pattern,
    expected_kind: Option<&ValueKind>,
    interpreter: &Interpreter,
) -> MResult<CompiledPattern> {
    PatternCompiler::default().compile(pattern, expected_kind, interpreter)
}

enum PatternExpressionSource<'a> {
    Interpreter {
        env: &'a Environment,
        interpreter: &'a Interpreter,
    },
    Sampled(&'a [Value]),
}

struct PatternMatchState<'a> {
    binding_specs: Vec<PatternBindingSpec>,
    proposed: Vec<Option<Value>>,
    expression_source: PatternExpressionSource<'a>,
}

impl PatternMatchState<'_> {
    fn expression_value(
        &self,
        expression_index: usize,
        expression_node: &Expression,
    ) -> MResult<Value> {
        match &self.expression_source {
            PatternExpressionSource::Interpreter { env, interpreter } => {
                // Expression patterns read the arm's outer environment. Proposed
                // captures are intentionally invisible; capture-dependent
                // conditions belong in an explicit arm guard.
                expression(expression_node, Some(env), interpreter)
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

    fn matches(&mut self, pattern: &CompiledPattern, value: &Value) -> MResult<bool> {
        let value = deep_detach_value(value);
        match pattern {
            CompiledPattern::Wildcard => Ok(true),
            CompiledPattern::Binding { binding_index, .. } => {
                if let Some(existing) = &self.proposed[*binding_index] {
                    Ok(existing == &value)
                } else {
                    self.proposed[*binding_index] = Some(value);
                    Ok(true)
                }
            }
            CompiledPattern::ExpressionValue {
                expression_index,
                expression,
            } => {
                let expected =
                    deep_detach_value(&self.expression_value(*expression_index, expression)?);
                Ok(values_match(&expected, &value))
            }
            CompiledPattern::Tuple { elements } => {
                #[cfg(feature = "tuple")]
                if let Value::Tuple(tuple) = value {
                    let tuple = tuple.borrow();
                    if tuple.elements.len() != elements.len() {
                        return Ok(false);
                    }
                    for (pattern, value) in elements.iter().zip(tuple.elements.iter()) {
                        if !self.matches(pattern, value)? {
                            return Ok(false);
                        }
                    }
                    return Ok(true);
                }
                Ok(false)
            }
            CompiledPattern::Array {
                prefix,
                spread,
                suffix,
            } => {
                #[cfg(feature = "matrix")]
                {
                    let values = match matrix_like_values(&value) {
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
                    if let Some(binding) =
                        spread.as_ref().and_then(|spread| spread.binding.as_deref())
                    {
                        let middle = capture_middle_matrix(&value, prefix.len(), suffix_start);
                        if !self.matches(binding, &middle)? {
                            return Ok(false);
                        }
                    }
                    return Ok(true);
                }
                #[cfg(not(feature = "matrix"))]
                Ok(false)
            }
            CompiledPattern::EnumVariant {
                enum_id,
                variant_id,
                payload,
            } => {
                #[cfg(feature = "enum")]
                if let Value::Enum(enum_value) = value {
                    let enum_value = enum_value.borrow();
                    if enum_id.is_some_and(|expected| expected != enum_value.id)
                        || enum_value.variants.len() != 1
                    {
                        return Ok(false);
                    }
                    let (actual_variant, actual_payload) = &enum_value.variants[0];
                    if actual_variant != variant_id {
                        return Ok(false);
                    }
                    return match (payload, actual_payload) {
                        (None, None) => Ok(true),
                        (Some(pattern), Some(value)) => self.matches(pattern, value),
                        _ => Ok(false),
                    };
                }
                Ok(false)
            }
            CompiledPattern::AtomTuple { tag_id, payload } => {
                #[cfg(feature = "enum")]
                if let Value::Enum(enum_value) = &value {
                    let enum_value = enum_value.borrow();
                    if enum_value.variants.len() != 1 {
                        return Ok(false);
                    }
                    let (actual_variant, actual_payload) = &enum_value.variants[0];
                    if actual_variant != tag_id {
                        return Ok(false);
                    }
                    return match (payload.as_slice(), actual_payload) {
                        ([], None) => Ok(true),
                        ([pattern], Some(value)) => self.matches(pattern, value),
                        _ => Ok(false),
                    };
                }
                #[cfg(all(feature = "tuple", feature = "atom"))]
                if let Value::Tuple(tuple) = &value {
                    let tuple = tuple.borrow();
                    if tuple.elements.len() != payload.len() + 1 {
                        return Ok(false);
                    }
                    let tag = deep_detach_value(&tuple.elements[0]);
                    let Value::Atom(tag) = tag else {
                        return Ok(false);
                    };
                    if tag.borrow().id() != *tag_id {
                        return Ok(false);
                    }
                    for (pattern, value) in payload.iter().zip(tuple.elements.iter().skip(1)) {
                        if !self.matches(pattern, value)? {
                            return Ok(false);
                        }
                    }
                    return Ok(true);
                }
                Ok(false)
            }
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
                    kind: value.kind().deref_kind(),
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
    value: &Value,
    env: &Environment,
    interpreter: &Interpreter,
) -> MResult<PatternMatch> {
    let specs = pattern.binding_specs();
    let mut state = PatternMatchState {
        proposed: vec![None; specs.len()],
        binding_specs: specs,
        expression_source: PatternExpressionSource::Interpreter { env, interpreter },
    };
    let matched = state.matches(pattern, value)?;
    Ok(state.finish(matched))
}

pub fn match_compiled_pattern_with_values(
    pattern: &CompiledPattern,
    value: &Value,
    expression_values: &[Value],
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
    args: &Vec<Value>,
    env: &mut Environment,
    interpreter: &Interpreter,
) -> MResult<bool> {
    if matches!(pattern, Pattern::Wildcard) {
        return Ok(true);
    }
    if args.len() == 1 {
        return pattern_matches_value(pattern, &args[0], env, interpreter);
    }
    #[cfg(feature = "tuple")]
    {
        let arguments = Value::Tuple(Ref::new(MechTuple::from_vec(args.clone())));
        return pattern_matches_value(pattern, &arguments, env, interpreter);
    }
    #[cfg(not(feature = "tuple"))]
    {
        Ok(args.is_empty() && matches!(pattern, Pattern::Wildcard))
    }
}

pub fn pattern_matches_value(
    pattern: &Pattern,
    value: &Value,
    env: &mut Environment,
    interpreter: &Interpreter,
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
pub fn pattern_to_value(pattern: &Pattern, env: &Environment, p: &Interpreter) -> MResult<Value> {
    match pattern {
        Pattern::Wildcard => Ok(Value::Empty),
        Pattern::Expression(expr) => expression(expr, Some(env), p),
        #[cfg(feature = "tuple")]
        Pattern::Tuple(pattern_tuple) => {
            let mut values = Vec::with_capacity(pattern_tuple.0.len());
            for inner in &pattern_tuple.0 {
                values.push(pattern_to_value(inner, env, p)?);
            }
            return Ok(Value::Tuple(Ref::new(MechTuple::from_vec(values))));
        }
        #[cfg(feature = "matrix")]
        Pattern::Array(array) => {
            let mut values = Vec::new();
            for inner in &array.prefix {
                let inner_value = pattern_to_value(inner, env, p)?;
                if let Some(inner_values) = matrix_like_values(&inner_value) {
                    values.extend(inner_values);
                } else {
                    values.push(inner_value);
                }
            }
            if let Some(spread) = &array.spread {
                if let Some(binding) = &spread.binding {
                    let bound = pattern_to_value(binding, env, p)?;
                    if let Some(bound_values) = matrix_like_values(&bound) {
                        values.extend(bound_values);
                    } else {
                        values.push(bound);
                    }
                }
            }
            for inner in &array.suffix {
                let inner_value = pattern_to_value(inner, env, p)?;
                if let Some(inner_values) = matrix_like_values(&inner_value) {
                    values.extend(inner_values);
                } else {
                    values.push(inner_value);
                }
            }
            return Ok(build_row_matrix_from_values(values));
        }
        #[cfg(all(feature = "tuple", feature = "atom"))]
        Pattern::TupleStruct(pattern_tuple_struct) => {
            #[cfg(feature = "enum")]
            {
                let variant_id = pattern_tuple_struct.name.hash();
                if let Some((enum_id, enum_def)) = p.state.borrow().enums.iter().find(|(_, enm)| {
                    enm.variants
                        .iter()
                        .any(|(known_variant, _)| *known_variant == variant_id)
                }) {
                    let payload = if pattern_tuple_struct.patterns.len() == 1 {
                        Some(pattern_to_value(&pattern_tuple_struct.patterns[0], env, p)?)
                    } else if pattern_tuple_struct.patterns.is_empty() {
                        None
                    } else {
                        return Err(MechError::new(FeatureNotEnabledError, Some("Enum tuple-struct patterns currently support zero or one payload value.".to_string())).with_compiler_loc());
                    };
                    let enm = MechEnum {
                        id: *enum_id,
                        variants: vec![(variant_id, payload)],
                        names: enum_def.names.clone(),
                    };
                    return Ok(Value::Enum(Ref::new(enm)));
                }
            }
            let mut values = Vec::with_capacity(pattern_tuple_struct.patterns.len() + 1);
            values.push(atom(
                &Atom {
                    name: pattern_tuple_struct.name.clone(),
                },
                p,
            ));
            for inner in &pattern_tuple_struct.patterns {
                values.push(pattern_to_value(inner, env, p)?);
            }
            return Ok(Value::Tuple(Ref::new(MechTuple::from_vec(values))));
        }
        _ => Err(MechError::new(FeatureNotEnabledError, None).with_compiler_loc()),
    }
}

// Mutable reference unwrapper. Recursively follows Value::MutableReference
// chains until it reaches a plain value, then clones it. Ensures the pattern
// matcher always works on an owned, non-reference value.
fn deep_detach_value(value: &Value) -> Value {
    match value {
        Value::MutableReference(reference) => deep_detach_value(&reference.borrow()),
        _ => value.clone(),
    }
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
fn capture_middle_matrix(value: &Value, start: usize, end: usize) -> Value {
    let cols = end.saturating_sub(start);
    match value {
        #[cfg(feature = "matrix")]
        Value::MatrixIndex(matrix) => Value::MatrixIndex(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "bool"))]
        Value::MatrixBool(matrix) => Value::MatrixBool(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "u8"))]
        Value::MatrixU8(matrix) => Value::MatrixU8(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "u16"))]
        Value::MatrixU16(matrix) => Value::MatrixU16(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "u32"))]
        Value::MatrixU32(matrix) => Value::MatrixU32(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "u64"))]
        Value::MatrixU64(matrix) => Value::MatrixU64(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "u128"))]
        Value::MatrixU128(matrix) => Value::MatrixU128(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "i8"))]
        Value::MatrixI8(matrix) => Value::MatrixI8(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "i16"))]
        Value::MatrixI16(matrix) => Value::MatrixI16(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "i32"))]
        Value::MatrixI32(matrix) => Value::MatrixI32(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "i64"))]
        Value::MatrixI64(matrix) => Value::MatrixI64(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "i128"))]
        Value::MatrixI128(matrix) => Value::MatrixI128(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "f32"))]
        Value::MatrixF32(matrix) => Value::MatrixF32(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "f64"))]
        Value::MatrixF64(matrix) => Value::MatrixF64(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "string"))]
        Value::MatrixString(matrix) => Value::MatrixString(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "rational"))]
        Value::MatrixR64(matrix) => Value::MatrixR64(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(all(feature = "matrix", feature = "complex"))]
        Value::MatrixC64(matrix) => Value::MatrixC64(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        #[cfg(feature = "matrix")]
        Value::MatrixValue(matrix) => Value::MatrixValue(Matrix::from_vec(
            matrix.as_vec()[start..end].to_vec(),
            1,
            cols,
        )),
        _ => {
            let values = matrix_like_values(value).unwrap_or_default();
            Value::MatrixValue(Matrix::from_vec(values[start..end].to_vec(), 1, cols))
        }
    }
}

// Used by the Array pattern arm to get a uniform element list regardless of the matrix's concrete numeric type.
pub(crate) fn matrix_like_values(value: &Value) -> Option<Vec<Value>> {
    match value {
        #[cfg(feature = "matrix")]
        Value::MatrixIndex(matrix) => Some(
            matrix
                .as_vec()
                .into_iter()
                .map(|value| Value::Index(Ref::new(value)))
                .collect(),
        ),
        #[cfg(all(feature = "matrix", feature = "bool"))]
        Value::MatrixBool(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "u8"))]
        Value::MatrixU8(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "u16"))]
        Value::MatrixU16(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "u32"))]
        Value::MatrixU32(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "u64"))]
        Value::MatrixU64(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "u128"))]
        Value::MatrixU128(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "i8"))]
        Value::MatrixI8(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "i16"))]
        Value::MatrixI16(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "i32"))]
        Value::MatrixI32(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "i64"))]
        Value::MatrixI64(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "i128"))]
        Value::MatrixI128(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "f32"))]
        Value::MatrixF32(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "f64"))]
        Value::MatrixF64(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "string"))]
        Value::MatrixString(matrix) => Some(matrix.as_vec().into_iter().map(Value::from).collect()),
        #[cfg(all(feature = "matrix", feature = "rational"))]
        Value::MatrixR64(matrix) => Some(
            matrix
                .as_vec()
                .into_iter()
                .map(|value| value.to_value())
                .collect(),
        ),
        #[cfg(all(feature = "matrix", feature = "complex"))]
        Value::MatrixC64(matrix) => Some(
            matrix
                .as_vec()
                .into_iter()
                .map(|value| value.to_value())
                .collect(),
        ),
        #[cfg(feature = "matrix")]
        Value::MatrixValue(matrix) => Some(matrix.as_vec()),
        _ => None,
    }
}

#[cfg(feature = "matrix")]
fn build_row_matrix_from_values(values: Vec<Value>) -> Value {
    let cols = values.len();
    #[cfg(feature = "u64")]
    if values.iter().all(|value| matches!(value, Value::U64(_))) {
        let data = values
            .iter()
            .map(|value| match value {
                Value::U64(x) => *x.borrow(),
                _ => unreachable!(),
            })
            .collect::<Vec<u64>>();
        return Value::MatrixU64(Matrix::from_vec(data, 1, cols));
    }
    Value::MatrixValue(Matrix::from_vec(values, 1, cols))
}

pub(crate) fn pattern_var_is_binding(var: &Var) -> bool {
    var.context.is_none()
        // Runtime context inputs are sampled pattern values, never captures.
        && !var
            .name
            .to_string()
            .starts_with(RUNTIME_CONTEXT_PATTERN_VALUE_PREFIX)
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

// TODO: This needs to be expanded to handle all types.
fn values_match(expected: &Value, actual: &Value) -> bool {
    if expected == actual {
        return true;
    }
    match (expected, actual) {
        #[cfg(all(feature = "atom", feature = "enum"))]
        (Value::Atom(atom), Value::Enum(enum_value))
        | (Value::Enum(enum_value), Value::Atom(atom)) => {
            let enum_value = enum_value.borrow();
            return enum_value.variants.len() == 1
                && enum_value.variants[0].0 == atom.borrow().id()
                && enum_value.variants[0].1.is_none();
        }
        #[cfg(all(feature = "u64", feature = "f64"))]
        (Value::F64(x), Value::U64(y)) => {
            let x = *x.borrow();
            return x.is_finite()
                && x >= 0.0
                && x.fract() == 0.0
                && x < 18_446_744_073_709_551_616.0
                && (x as u64) == *y.borrow();
        }
        #[cfg(all(feature = "u64", feature = "f64"))]
        (Value::U64(x), Value::F64(y)) => {
            let y = *y.borrow();
            return y.is_finite()
                && y >= 0.0
                && y.fract() == 0.0
                && y < 18_446_744_073_709_551_616.0
                && *x.borrow() == (y as u64);
        }
        _ => {}
    }
    false
}

#[cfg(all(test, feature = "tuple", feature = "u64"))]
mod tests {
    use super::*;

    #[test]
    fn failed_compiled_match_returns_no_bindings_and_cannot_mutate_sink() {
        let id = hash_str("x");
        let binding = CompiledPattern::Binding {
            binding_index: 0,
            id,
            name: "x".to_string(),
            expected_kind: Some(ValueKind::U64),
        };
        let pattern = CompiledPattern::Tuple {
            elements: vec![binding.clone(), binding],
        };
        let value = Value::Tuple(Ref::new(MechTuple::from_vec(vec![
            Value::U64(Ref::new(1)),
            Value::U64(Ref::new(2)),
        ])));

        let pattern_match = match_compiled_pattern_with_values(&pattern, &value, &[]).unwrap();
        assert!(!pattern_match.matched);
        assert!(pattern_match.bindings.is_empty());

        let mut env = Environment::new();
        env.insert(id, Value::U64(Ref::new(9)));
        EnvironmentBindingSink::new(&mut env)
            .commit(&pattern_match)
            .unwrap();
        assert_eq!(env.get(&id), Some(&Value::U64(Ref::new(9))));
    }
}
