#![cfg(all(feature = "functions", feature = "symbol_table"))]
//! Statically elaborated structural dispatch for patterned activation scopes.
use crate::*;

macro_rules! activation_error {
    ($n:ident,$m:expr) => {
        #[derive(Debug, Clone)]
        pub(crate) struct $n;
        impl MechErrorKind for $n {
            fn name(&self) -> &str {
                stringify!($n)
            }
            fn message(&self) -> String {
                $m.into()
            }
        }
    };
}
activation_error!(
    ActivationPatternExpressionUnsupported,
    "This activation pattern is not supported."
);
activation_error!(
    ActivationPatternCaptureKindUnsupported,
    "The capture kind cannot be inferred from the activation trigger."
);
activation_error!(
    ActivationPatternArmsNonExhaustive,
    "Patterned activations require a final unguarded wildcard arm."
);
activation_error!(
    ActivationPatternWildcardMustBeLast,
    "The wildcard activation arm must be last."
);
activation_error!(
    ActivationPatternGuardMustBePure,
    "Patterned activation guards are not supported yet."
);
activation_error!(
    ActivationPatternRegisterWriteUnsupported,
    "Patterned activation register writes are not supported."
);
activation_error!(
    ActivationPatternContextEffectUnsupported,
    "Patterned activation context effects are not supported."
);
activation_error!(
    ActivationPatternTriggerInvariant,
    "Activation trigger root cells disagree with the resolved trigger."
);

#[derive(Clone, Debug)]
enum CompiledActivationPattern {
    Wildcard,
    Literal {
        expected: Value,
    },
    Capture {
        capture_index: usize,
    },
    Tuple {
        elements: Vec<CompiledActivationPattern>,
    },
    EnumVariant {
        enum_id: u64,
        variant_id: u64,
        payload: Option<Box<CompiledActivationPattern>>,
    },
    AtomTuple {
        tag_id: u64,
        payload: Vec<CompiledActivationPattern>,
    },
}
#[derive(Clone)]
struct ActivationPatternCapture {
    id: u64,
    name: String,
    kind: ValueKind,
    slot: Value,
}
#[derive(Clone)]
struct PreflightActivationArm {
    pattern: CompiledActivationPattern,
    captures: Vec<ActivationPatternCapture>,
}
struct PreflightPatternedActivation {
    trigger_kind: ValueKind,
    arms: Vec<PreflightActivationArm>,
}
#[derive(Debug, Clone)]
pub(crate) struct ActivationPatternDefinitionUnsupported;
impl MechErrorKind for ActivationPatternDefinitionUnsupported {
    fn name(&self) -> &str {
        "ActivationPatternDefinitionUnsupported"
    }
    fn message(&self) -> String {
        "This definition or declaration is not supported inside a patterned activation arm."
            .to_string()
    }
}
fn detached(v: &Value) -> Value {
    match v {
        Value::MutableReference(r) => detached(&r.borrow()),
        _ => v.clone(),
    }
}
fn clone_ref_value<T: Clone>(destination: &Ref<T>, source: &Ref<T>) {
    destination.borrow_mut().clone_from(&*source.borrow())
}
fn create_capture_slot_for_kind(kind: &ValueKind) -> MResult<Value> {
    match kind.deref_kind() {
        #[cfg(feature = "u8")]
        ValueKind::U8 => Ok(Value::U8(Ref::new(0))),
        #[cfg(feature = "u16")]
        ValueKind::U16 => Ok(Value::U16(Ref::new(0))),
        #[cfg(feature = "u32")]
        ValueKind::U32 => Ok(Value::U32(Ref::new(0))),
        #[cfg(feature = "u64")]
        ValueKind::U64 => Ok(Value::U64(Ref::new(0))),
        #[cfg(feature = "u128")]
        ValueKind::U128 => Ok(Value::U128(Ref::new(0))),
        #[cfg(feature = "i8")]
        ValueKind::I8 => Ok(Value::I8(Ref::new(0))),
        #[cfg(feature = "i16")]
        ValueKind::I16 => Ok(Value::I16(Ref::new(0))),
        #[cfg(feature = "i32")]
        ValueKind::I32 => Ok(Value::I32(Ref::new(0))),
        #[cfg(feature = "i64")]
        ValueKind::I64 => Ok(Value::I64(Ref::new(0))),
        #[cfg(feature = "i128")]
        ValueKind::I128 => Ok(Value::I128(Ref::new(0))),
        #[cfg(feature = "f64")]
        ValueKind::F64 => Ok(Value::F64(Ref::new(0.0))),
        #[cfg(feature = "f32")]
        ValueKind::F32 => Ok(Value::F32(Ref::new(0.0))),
        #[cfg(feature = "complex")]
        ValueKind::C64 => Ok(Value::C64(Ref::new(C64::default()))),
        #[cfg(feature = "rational")]
        ValueKind::R64 => Ok(Value::R64(Ref::new(R64::default()))),
        #[cfg(any(feature = "bool", feature = "variable_define"))]
        ValueKind::Bool => Ok(Value::Bool(Ref::new(false))),
        #[cfg(any(feature = "string", feature = "variable_define"))]
        ValueKind::String => Ok(Value::String(Ref::new(String::new()))),
        ValueKind::Index => Ok(Value::Index(Ref::new(0))),
        #[cfg(feature = "atom")]
        ValueKind::Atom(id, _) => Ok(Value::Atom(Ref::new(MechAtom::new(id)))),
        _ => Err(MechError::new(
            ActivationPatternCaptureKindUnsupported,
            None,
        )),
    }
}
fn commit_capture_slot(destination: &Value, source: &Value) -> MResult<()> {
    match (destination, &detached(source)) {
        #[cfg(feature = "u8")]
        (Value::U8(a), Value::U8(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "u16")]
        (Value::U16(a), Value::U16(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "u32")]
        (Value::U32(a), Value::U32(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "u64")]
        (Value::U64(a), Value::U64(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "u128")]
        (Value::U128(a), Value::U128(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "i8")]
        (Value::I8(a), Value::I8(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "i16")]
        (Value::I16(a), Value::I16(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "i32")]
        (Value::I32(a), Value::I32(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "i64")]
        (Value::I64(a), Value::I64(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "i128")]
        (Value::I128(a), Value::I128(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "f64")]
        (Value::F64(a), Value::F64(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "f32")]
        (Value::F32(a), Value::F32(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "complex")]
        (Value::C64(a), Value::C64(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "rational")]
        (Value::R64(a), Value::R64(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(any(feature = "bool", feature = "variable_define"))]
        (Value::Bool(a), Value::Bool(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(any(feature = "string", feature = "variable_define"))]
        (Value::String(a), Value::String(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        (Value::Index(a), Value::Index(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        #[cfg(feature = "atom")]
        (Value::Atom(a), Value::Atom(b)) => {
            clone_ref_value(a, b);
            Ok(())
        }
        _ => Err(MechError::new(
            ActivationPatternCaptureKindUnsupported,
            None,
        )),
    }
}
fn compile_activation_literal(l: &Literal, i: &Interpreter) -> MResult<Value> {
    match l {
        Literal::Empty(_) => Ok(Value::Empty),
        #[cfg(feature = "bool")]
        Literal::Boolean(t) => Ok(crate::boolean(t)),
        #[cfg(feature = "string")]
        Literal::String(v) => Ok(crate::string(v)),
        #[cfg(feature = "atom")]
        Literal::Atom(a) => Ok(Value::Atom(Ref::new(MechAtom::new(a.name.hash())))),
        Literal::Number(Number::Real(RealNumber::TypedInteger(_)))
        | Literal::TypedLiteral(_)
        | Literal::Kind(_) => Err(MechError::new(ActivationPatternExpressionUnsupported, None)),
        Literal::Number(n) => crate::number(n, i),
        _ => Err(MechError::new(ActivationPatternExpressionUnsupported, None)),
    }
}
#[cfg(feature = "enum")]
fn compile_enum_variant_pattern(
    t: &PatternTupleStruct,
    enum_id: u64,
    i: &Interpreter,
    c: &mut Vec<ActivationPatternCapture>,
) -> MResult<CompiledActivationPattern> {
    let variant_id = t.name.hash();
    let def = i
        .state
        .borrow()
        .enums
        .get(&enum_id)
        .cloned()
        .ok_or_else(|| {
            MechError::new(ActivationPatternCaptureKindUnsupported, None).with_tokens(t.tokens())
        })?;
    let declared = def
        .variants
        .iter()
        .find(|(id, _)| *id == variant_id)
        .map(|(_, p)| p.clone())
        .ok_or_else(|| {
            MechError::new(ActivationPatternCaptureKindUnsupported, None).with_tokens(t.tokens())
        })?;
    let payload = match (t.patterns.as_slice(), declared) {
        ([], None) => None,
        ([p], Some(Value::Kind(k))) => Some(Box::new(compile_activation_pattern(p, &k, i, c)?)),
        _ => {
            return Err(
                MechError::new(ActivationPatternCaptureKindUnsupported, None)
                    .with_tokens(t.tokens()),
            );
        }
    };
    Ok(CompiledActivationPattern::EnumVariant {
        enum_id,
        variant_id,
        payload,
    })
}
#[cfg(all(feature = "tuple", feature = "atom"))]
fn compile_atom_tuple_pattern(
    t: &PatternTupleStruct,
    kinds: &[ValueKind],
    i: &Interpreter,
    c: &mut Vec<ActivationPatternCapture>,
) -> MResult<CompiledActivationPattern> {
    let Some(first) = kinds.first() else {
        return Err(
            MechError::new(ActivationPatternCaptureKindUnsupported, None).with_tokens(t.tokens()),
        );
    };
    if !matches!(first.deref_kind(), ValueKind::Atom(_, _)) || t.patterns.len() != kinds.len() - 1 {
        return Err(
            MechError::new(ActivationPatternCaptureKindUnsupported, None).with_tokens(t.tokens()),
        );
    };
    let payload = t
        .patterns
        .iter()
        .zip(kinds.iter().skip(1))
        .map(|(p, k)| compile_activation_pattern(p, k, i, c))
        .collect::<MResult<_>>()?;
    Ok(CompiledActivationPattern::AtomTuple {
        tag_id: t.name.hash(),
        payload,
    })
}
fn compile_activation_pattern(
    p: &Pattern,
    kind: &ValueKind,
    i: &Interpreter,
    c: &mut Vec<ActivationPatternCapture>,
) -> MResult<CompiledActivationPattern> {
    let kind = kind.deref_kind();
    match p {
        Pattern::Wildcard => Ok(CompiledActivationPattern::Wildcard),
        Pattern::Expression(Expression::Literal(l)) => {
            let expected = compile_activation_literal(l, i)?;
            if expected.kind().deref_kind() != kind {
                return Err(
                    MechError::new(ActivationPatternCaptureKindUnsupported, None)
                        .with_tokens(p.tokens()),
                );
            }
            Ok(CompiledActivationPattern::Literal { expected })
        }
        Pattern::Expression(Expression::Var(v)) => {
            let id = v.name.hash();
            if let Some(n) = c.iter().position(|x| x.id == id) {
                if c[n].kind.deref_kind() != kind {
                    return Err(
                        MechError::new(ActivationPatternCaptureKindUnsupported, None)
                            .with_tokens(p.tokens()),
                    );
                }
                return Ok(CompiledActivationPattern::Capture { capture_index: n });
            }
            let slot =
                create_capture_slot_for_kind(&kind).map_err(|e| e.with_tokens(p.tokens()))?;
            c.push(ActivationPatternCapture {
                id,
                name: v.name.to_string(),
                kind,
                slot,
            });
            Ok(CompiledActivationPattern::Capture {
                capture_index: c.len() - 1,
            })
        }
        #[cfg(feature = "tuple")]
        Pattern::Tuple(t) => {
            let ValueKind::Tuple(k) = kind else {
                return Err(
                    MechError::new(ActivationPatternCaptureKindUnsupported, None)
                        .with_tokens(p.tokens()),
                );
            };
            if t.0.len() != k.len() {
                return Err(
                    MechError::new(ActivationPatternCaptureKindUnsupported, None)
                        .with_tokens(p.tokens()),
                );
            }
            Ok(CompiledActivationPattern::Tuple {
                elements: t
                    .0
                    .iter()
                    .zip(k.iter())
                    .map(|(p, k)| compile_activation_pattern(p, k, i, c))
                    .collect::<MResult<_>>()?,
            })
        }
        Pattern::TupleStruct(t) => match kind {
            #[cfg(feature = "enum")]
            ValueKind::Enum(id, _) => compile_enum_variant_pattern(t, id, i, c),
            #[cfg(all(feature = "tuple", feature = "atom"))]
            ValueKind::Tuple(k) => compile_atom_tuple_pattern(t, &k, i, c),
            _ => Err(
                MechError::new(ActivationPatternCaptureKindUnsupported, None)
                    .with_tokens(p.tokens()),
            ),
        },
        _ => {
            Err(MechError::new(ActivationPatternExpressionUnsupported, None)
                .with_tokens(p.tokens()))
        }
    }
}
fn matches_pattern(
    p: &CompiledActivationPattern,
    v: &Value,
    proposed: &mut Vec<Option<Value>>,
) -> bool {
    match p {
        CompiledActivationPattern::Wildcard => true,
        CompiledActivationPattern::Literal { expected } => expected == &detached(v),
        CompiledActivationPattern::Capture { capture_index } => {
            let x = detached(v);
            match &proposed[*capture_index] {
                Some(y) => y == &x,
                None => {
                    proposed[*capture_index] = Some(x);
                    true
                }
            }
        }
        #[cfg(feature = "tuple")]
        CompiledActivationPattern::Tuple { elements } => match detached(v) {
            Value::Tuple(t) => {
                let t = t.borrow();
                t.elements.len() == elements.len()
                    && elements
                        .iter()
                        .zip(t.elements.iter())
                        .all(|(p, v)| matches_pattern(p, v, proposed))
            }
            _ => false,
        },
        CompiledActivationPattern::EnumVariant {
            enum_id,
            variant_id,
            payload,
        } => {
            #[cfg(feature = "enum")]
            {
                let Value::Enum(e) = detached(v) else {
                    return false;
                };
                let e = e.borrow();
                if e.id != *enum_id || e.variants.len() != 1 {
                    return false;
                }
                let (id, value) = &e.variants[0];
                id == variant_id
                    && match (payload, value) {
                        (None, None) => true,
                        (Some(p), Some(v)) => matches_pattern(p, v, proposed),
                        _ => false,
                    }
            }
            #[cfg(not(feature = "enum"))]
            {
                false
            }
        }
        CompiledActivationPattern::AtomTuple { tag_id, payload } => {
            #[cfg(all(feature = "tuple", feature = "atom"))]
            {
                let Value::Tuple(t) = detached(v) else {
                    return false;
                };
                let t = t.borrow();
                if t.elements.len() != payload.len() + 1 {
                    return false;
                }
                let Value::Atom(tag) = detached(&t.elements[0]) else {
                    return false;
                };
                tag.borrow().id() == *tag_id
                    && payload
                        .iter()
                        .zip(t.elements.iter().skip(1))
                        .all(|(p, v)| matches_pattern(p, v, proposed))
            }
            #[cfg(not(all(feature = "tuple", feature = "atom")))]
            {
                false
            }
        }
        #[cfg(not(feature = "tuple"))]
        CompiledActivationPattern::Tuple { .. } => false,
    }
}
fn generation() -> (Ref<usize>, Value) {
    let r = Ref::new(0);
    (r.clone(), Value::Index(r))
}
struct ScopePulse {
    out: Ref<usize>,
}
impl MechFunctionImpl for ScopePulse {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        *self.out.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn reactive_dependency_scopes(&self, _: usize) -> Option<Vec<ReactiveDependencyScope>> {
        Some(vec![ReactiveDependencyScope::Root])
    }
    fn to_string(&self) -> String {
        "ActivationPatternScopePulse".into()
    }
}
struct Matcher {
    pattern: CompiledActivationPattern,
    trigger: Value,
    captures: Vec<ActivationPatternCapture>,
    matched: Ref<bool>,
    out: Ref<usize>,
}
impl MechFunctionImpl for Matcher {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        let mut p = vec![None; self.captures.len()];
        let ok = matches_pattern(&self.pattern, &self.trigger, &mut p);
        if ok {
            for (c, v) in self.captures.iter().zip(p.iter()) {
                if let Some(v) = v {
                    commit_capture_slot(&c.slot, v)?
                }
            }
        }
        *self.matched.borrow_mut() = ok;
        *self.out.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn reactive_dependency_kinds(&self, _: usize) -> Option<Vec<ReactiveDependencyKind>> {
        Some(vec![
            ReactiveDependencyKind::Reactive,
            ReactiveDependencyKind::Sampled,
        ])
    }
    fn to_string(&self) -> String {
        "ActivationPatternMatcher".into()
    }
}
struct Finalize {
    matched: Ref<bool>,
    eligible: Ref<bool>,
    out: Ref<usize>,
}
impl MechFunctionImpl for Finalize {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        *self.eligible.borrow_mut() = *self.matched.borrow();
        *self.out.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn to_string(&self) -> String {
        "ActivationPatternArmFinalize".into()
    }
}
struct Select {
    eligible: Vec<Ref<bool>>,
    selected: Ref<usize>,
    out: Ref<usize>,
}
impl MechFunctionImpl for Select {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        *self.selected.borrow_mut() = self
            .eligible
            .iter()
            .position(|x| *x.borrow())
            .unwrap_or(usize::MAX);
        *self.out.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn to_string(&self) -> String {
        "ActivationPatternSelectArm".into()
    }
}
struct Gate {
    arm: usize,
    selected: Ref<usize>,
    out: Ref<usize>,
}
impl MechFunctionImpl for Gate {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        if *self.selected.borrow() == self.arm {
            *self.out.borrow_mut() += 1;
            Ok(ReactiveSolveStatus::Changed)
        } else {
            Ok(ReactiveSolveStatus::Unchanged)
        }
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn to_string(&self) -> String {
        "ActivationPatternArmGate".into()
    }
}

#[cfg(feature = "compiler")]
macro_rules! interpreter_only {
    ($t:ty) => {
        impl MechFunctionCompiler for $t {
            fn compile(&self, _: &mut CompileCtx) -> MResult<Register> {
                Err(MechError::new(
                    GenericError {
                        msg: "Activation pattern dispatch is interpreter-only.".into(),
                    },
                    None,
                ))
            }
        }
    };
}
#[cfg(feature = "compiler")]
interpreter_only!(ScopePulse);
#[cfg(feature = "compiler")]
interpreter_only!(Matcher);
#[cfg(feature = "compiler")]
interpreter_only!(Finalize);
#[cfg(feature = "compiler")]
interpreter_only!(Select);
#[cfg(feature = "compiler")]
interpreter_only!(Gate);

fn preflight_patterned_activation(
    scope: &ActivationScope,
    arms: &[ActivationArm],
    trigger: &Value,
    trigger_cells: &[ReactiveCellId],
    i: &Interpreter,
) -> MResult<PreflightPatternedActivation> {
    let last = arms.last().ok_or_else(|| {
        MechError::new(ActivationPatternArmsNonExhaustive, None).with_tokens(scope.tokens())
    })?;
    if !matches!(last.pattern, Pattern::Wildcard) || last.guard.is_some() {
        return Err(
            MechError::new(ActivationPatternArmsNonExhaustive, None).with_tokens(scope.tokens())
        );
    }
    if arms[..arms.len() - 1]
        .iter()
        .any(|a| matches!(a.pattern, Pattern::Wildcard))
    {
        return Err(
            MechError::new(ActivationPatternWildcardMustBeLast, None).with_tokens(scope.tokens())
        );
    }
    if arms.iter().any(|a| a.guard.is_some()) {
        return Err(
            MechError::new(ActivationPatternGuardMustBePure, None).with_tokens(scope.tokens())
        );
    }
    for arm in arms {
        validate_patterned_arm_body(&arm.body)?;
    }
    if trigger.reactive_root_cell_ids() != trigger_cells {
        return Err(
            MechError::new(ActivationPatternTriggerInvariant, None).with_tokens(scope.tokens())
        );
    }
    let trigger_kind = trigger.kind().deref_kind();
    let mut compiled = Vec::new();
    for a in arms {
        let mut captures = Vec::new();
        let pattern = compile_activation_pattern(&a.pattern, &trigger_kind, i, &mut captures)?;
        compiled.push(PreflightActivationArm { pattern, captures });
    }
    Ok(PreflightPatternedActivation {
        trigger_kind,
        arms: compiled,
    })
}

fn validation_error(kind: impl MechErrorKind + 'static, tokens: Vec<Token>) -> MResult<()> {
    Err(MechError::new(kind, None).with_tokens(tokens))
}

fn validate_patterned_arm_body(body: &ActivationArmBody) -> MResult<()> {
    match body {
        ActivationArmBody::Block(body) => {
            for (code, _) in body {
                validate_patterned_code(code)?;
            }
            Ok(())
        }
        ActivationArmBody::Expression(expression) => validate_patterned_expression(expression),
    }
}
fn validate_patterned_code(code: &MechCode) -> MResult<()> {
    match code {
        MechCode::Comment(_) => Ok(()),
        MechCode::Expression(expression) => validate_patterned_expression(expression),
        MechCode::Statement(statement) => validate_patterned_statement(statement),
        MechCode::ActivationScope(_)
        | MechCode::FunctionDefine(_)
        | MechCode::FsmSpecification(_)
        | MechCode::FsmImplementation(_)
        | MechCode::Import(_)
        | MechCode::Error(_, _) => {
            validation_error(ActivationPatternDefinitionUnsupported, code.tokens())
        }
    }
}
fn validate_patterned_statement(statement: &Statement) -> MResult<()> {
    match statement {
        Statement::VariableDefine(definition)
            if !definition.mutable && definition.var.context.is_none() =>
        {
            validate_patterned_expression(&definition.expression)
        }
        Statement::VariableDefine(definition) if definition.var.context.is_some() => {
            validation_error(
                ActivationPatternContextEffectUnsupported,
                statement.tokens(),
            )
        }
        Statement::VariableDefine(_) => {
            validation_error(ActivationPatternDefinitionUnsupported, statement.tokens())
        }
        Statement::VariableAssign(assignment) if assignment.target.context.is_some() => {
            validation_error(
                ActivationPatternContextEffectUnsupported,
                statement.tokens(),
            )
        }
        Statement::VariableAssign(_) => validation_error(
            ActivationPatternRegisterWriteUnsupported,
            statement.tokens(),
        ),
        Statement::OpAssign(assignment) if assignment.target.context.is_some() => validation_error(
            ActivationPatternContextEffectUnsupported,
            statement.tokens(),
        ),
        Statement::OpAssign(_) => validation_error(
            ActivationPatternRegisterWriteUnsupported,
            statement.tokens(),
        ),
        Statement::ContextSend(_) => validation_error(
            ActivationPatternContextEffectUnsupported,
            statement.tokens(),
        ),
        _ => validation_error(ActivationPatternDefinitionUnsupported, statement.tokens()),
    }
}
fn validate_patterned_expression(expression: &Expression) -> MResult<()> {
    match expression {
        Expression::Literal(_) | Expression::Var(_) => Ok(()),
        Expression::Slice(slice) => validate_patterned_slice(slice),
        Expression::Formula(factor) => validate_patterned_factor(factor),
        Expression::FunctionCall(call) => {
            for (_, expression) in &call.args {
                validate_patterned_expression(expression)?;
            }
            Ok(())
        }
        Expression::Match(matched) => {
            validate_patterned_expression(&matched.source)?;
            for arm in &matched.arms {
                validate_patterned_pattern(&arm.pattern)?;
                if let Some(guard) = &arm.guard {
                    validate_patterned_expression(guard)?;
                }
                validate_patterned_expression(&arm.expression)?;
            }
            Ok(())
        }
        Expression::Range(range) => validate_patterned_range(range),
        Expression::Structure(structure) => validate_patterned_structure(structure),
        Expression::SetComprehension(comprehension) => {
            validate_patterned_expression(&comprehension.expression)?;
            for qualifier in &comprehension.qualifiers {
                validate_patterned_qualifier(qualifier)?;
            }
            Ok(())
        }
        Expression::MatrixComprehension(comprehension) => {
            validate_patterned_expression(&comprehension.expression)?;
            for qualifier in &comprehension.qualifiers {
                validate_patterned_qualifier(qualifier)?;
            }
            Ok(())
        }
        Expression::FsmPipe(_) => {
            validation_error(ActivationPatternDefinitionUnsupported, expression.tokens())
        }
    }
}
fn validate_patterned_pattern(pattern: &Pattern) -> MResult<()> {
    match pattern {
        Pattern::Expression(expression) => validate_patterned_expression(expression),
        Pattern::Tuple(tuple) => {
            for pattern in &tuple.0 {
                validate_patterned_pattern(pattern)?;
            }
            Ok(())
        }
        Pattern::TupleStruct(tuple) => {
            for pattern in &tuple.patterns {
                validate_patterned_pattern(pattern)?;
            }
            Ok(())
        }
        Pattern::Array(array) => {
            for pattern in array.prefix.iter().chain(&array.suffix) {
                validate_patterned_pattern(pattern)?;
            }
            if let Some(spread) = &array.spread {
                if let Some(binding) = &spread.binding {
                    validate_patterned_pattern(binding)?;
                }
            }
            Ok(())
        }
        Pattern::Wildcard => Ok(()),
    }
}
fn validate_patterned_factor(factor: &Factor) -> MResult<()> {
    match factor {
        Factor::Expression(expression) => validate_patterned_expression(expression),
        Factor::Negate(factor)
        | Factor::Not(factor)
        | Factor::Parenthetical(factor)
        | Factor::Transpose(factor) => validate_patterned_factor(factor),
        Factor::Term(term) => {
            validate_patterned_factor(&term.lhs)?;
            for (_, factor) in &term.rhs {
                validate_patterned_factor(factor)?;
            }
            Ok(())
        }
    }
}
fn validate_patterned_range(range: &RangeExpression) -> MResult<()> {
    validate_patterned_factor(&range.start)?;
    if let Some((_, increment)) = &range.increment {
        validate_patterned_factor(increment)?;
    }
    validate_patterned_factor(&range.terminal)
}
fn validate_patterned_slice(slice: &Slice) -> MResult<()> {
    for subscript in &slice.subscript {
        validate_patterned_subscript(subscript)?;
    }
    Ok(())
}
fn validate_patterned_subscript(subscript: &Subscript) -> MResult<()> {
    match subscript {
        Subscript::Brace(subscripts) | Subscript::Bracket(subscripts) => {
            for subscript in subscripts {
                validate_patterned_subscript(subscript)?;
            }
            Ok(())
        }
        Subscript::Formula(factor) => validate_patterned_factor(factor),
        Subscript::Range(range) => validate_patterned_range(range),
        Subscript::All | Subscript::Dot(_) | Subscript::DotInt(_) | Subscript::Swizzle(_) => Ok(()),
    }
}
fn validate_patterned_structure(structure: &Structure) -> MResult<()> {
    match structure {
        Structure::Empty => Ok(()),
        Structure::Map(map) => {
            for mapping in &map.elements {
                validate_patterned_expression(&mapping.key)?;
                validate_patterned_expression(&mapping.value)?;
            }
            Ok(())
        }
        Structure::Matrix(matrix) => {
            for row in &matrix.rows {
                for column in &row.columns {
                    validate_patterned_expression(&column.element)?;
                }
            }
            Ok(())
        }
        Structure::Record(record) => {
            for binding in &record.bindings {
                validate_patterned_expression(&binding.value)?;
            }
            Ok(())
        }
        Structure::Set(set) => {
            for expression in &set.elements {
                validate_patterned_expression(expression)?;
            }
            Ok(())
        }
        Structure::Table(table) => {
            for row in &table.rows {
                for column in &row.columns {
                    validate_patterned_expression(&column.element)?;
                }
            }
            Ok(())
        }
        Structure::Tuple(tuple) => {
            for expression in &tuple.elements {
                validate_patterned_expression(expression)?;
            }
            Ok(())
        }
        Structure::TupleStruct(tuple) => validate_patterned_expression(&tuple.value),
    }
}
fn validate_patterned_qualifier(qualifier: &ComprehensionQualifier) -> MResult<()> {
    match qualifier {
        ComprehensionQualifier::Generator((pattern, expression)) => {
            validate_patterned_pattern(pattern)?;
            validate_patterned_expression(expression)
        }
        ComprehensionQualifier::Filter(expression) => validate_patterned_expression(expression),
        ComprehensionQualifier::Let(definition) if definition.mutable => {
            validation_error(ActivationPatternDefinitionUnsupported, definition.tokens())
        }
        ComprehensionQualifier::Let(definition) if definition.var.context.is_some() => {
            validation_error(
                ActivationPatternContextEffectUnsupported,
                definition.tokens(),
            )
        }
        ComprehensionQualifier::Let(definition) => {
            validate_patterned_expression(&definition.expression)
        }
    }
}

fn elaborate_patterned_arm_body(
    arm: &ActivationArm,
    captures: &[ActivationPatternCapture],
    pulse: &Value,
    interpreter: &Interpreter,
) -> MResult<(usize, usize)> {
    let symbols = interpreter.symbols();
    let symbol_snapshot = symbols.borrow().snapshot();
    let plan = interpreter.plan();
    let original_scope_depth = plan.activation_registration_depth();
    {
        let mut symbols = symbols.borrow_mut();
        for capture in captures {
            symbols.mutable_variables.remove(&capture.id);
            symbols.insert(capture.id, capture.slot.clone(), false);
            symbols
                .dictionary
                .borrow_mut()
                .insert(capture.id, capture.name.clone());
        }
    }
    let body_node_start = plan.len();
    plan.push_activation_registration_scope(pulse.reactive_root_cell_ids());
    let body_result = (|| -> MResult<()> {
        match &arm.body {
            ActivationArmBody::Block(body) => {
                for (code, _) in body {
                    crate::mech_code(code, interpreter)?;
                }
                Ok(())
            }
            ActivationArmBody::Expression(expression) => {
                crate::expression(expression, None, interpreter)?;
                Ok(())
            }
        }
    })();
    while plan.activation_registration_depth() > original_scope_depth {
        plan.pop_activation_registration_scope();
    }
    symbols.borrow_mut().restore(symbol_snapshot);
    body_result?;
    Ok((body_node_start, plan.len()))
}

fn elaborate_patterned_activation_inner(
    arms: &[ActivationArm],
    trigger: Value,
    preflight: PreflightPatternedActivation,
    i: &Interpreter,
) -> MResult<Value> {
    if trigger.kind().deref_kind() != preflight.trigger_kind {
        return Err(MechError::new(ActivationPatternTriggerInvariant, None));
    }
    let compiled = preflight.arms;
    let plan = i.plan();
    let (scope_gen, scope_v) = generation();
    let scope_node = plan
        .borrow_mut()
        .register(Box::new(ScopePulse { out: scope_gen }), &[trigger.clone()])?;
    let (mut matcher_nodes, mut completions, mut matched) = (Vec::new(), Vec::new(), Vec::new());
    for arm in &compiled {
        let (o, v) = generation();
        let f = Ref::new(false);
        let n = plan.borrow_mut().register(
            Box::new(Matcher {
                pattern: arm.pattern.clone(),
                trigger: trigger.clone(),
                captures: arm.captures.clone(),
                matched: f.clone(),
                out: o,
            }),
            &[scope_v.clone(), trigger.clone()],
        )?;
        matcher_nodes.push(n);
        completions.push(v);
        matched.push(f);
    }
    let (mut finalizers, mut eligible, mut done) = (Vec::new(), Vec::new(), Vec::new());
    for (f, c) in matched.iter().zip(completions.iter()) {
        let (o, v) = generation();
        let e = Ref::new(false);
        finalizers.push(plan.borrow_mut().register(
            Box::new(Finalize {
                matched: f.clone(),
                eligible: e.clone(),
                out: o,
            }),
            &[c.clone()],
        )?);
        eligible.push(e);
        done.push(v);
    }
    let (o, selection) = generation();
    let selected = Ref::new(usize::MAX);
    let selector = plan.borrow_mut().register(
        Box::new(Select {
            eligible: eligible.clone(),
            selected: selected.clone(),
            out: o,
        }),
        &done,
    )?;
    let (mut gates, mut pulses) = (Vec::new(), Vec::new());
    for arm in 0..arms.len() {
        let (o, v) = generation();
        gates.push(plan.borrow_mut().register(
            Box::new(Gate {
                arm,
                selected: selected.clone(),
                out: o,
            }),
            &[selection.clone()],
        )?);
        pulses.push(v);
    }
    let mut ranges = Vec::new();
    for (arm, compiled_arm) in arms.iter().zip(&compiled) {
        ranges.push(elaborate_patterned_arm_body(
            arm,
            &compiled_arm.captures,
            &pulses[ranges.len()],
            i,
        )?);
    }
    let registration = PatternActivationRegistration {
        scope_pulse_node: scope_node,
        selector_node: selector,
        arms: (0..arms.len())
            .map(|n| PatternActivationArmRegistration {
                matcher_node: matcher_nodes[n],
                finalizer_node: finalizers[n],
                gate_node: gates[n],
                pulse_cell: pulses[n].reactive_root_cell_ids()[0],
                body_node_start: ranges[n].0,
                body_node_end: ranges[n].1,
                captures: compiled[n]
                    .captures
                    .iter()
                    .map(|c| PatternActivationCaptureRegistration {
                        id: c.id,
                        kind: c.kind.clone(),
                        cell: c.slot.reactive_root_cell_ids()[0],
                    })
                    .collect(),
            })
            .collect(),
    };
    plan.borrow_mut().register_pattern_activation(registration);
    Ok(Value::Empty)
}

pub(crate) fn elaborate_patterned_activation(
    scope: &ActivationScope,
    arms: &[ActivationArm],
    trigger: Value,
    trigger_cells: Vec<ReactiveCellId>,
    interpreter: &Interpreter,
) -> MResult<Value> {
    let preflight =
        preflight_patterned_activation(scope, arms, &trigger, &trigger_cells, interpreter)?;
    let plan = interpreter.plan();
    let checkpoint = plan.checkpoint();
    let program_dictionary = interpreter.state.borrow().dictionary.clone();
    let dictionary_snapshot = program_dictionary.borrow().clone();
    match elaborate_patterned_activation_inner(arms, trigger, preflight, interpreter) {
        Ok(value) => Ok(value),
        Err(error) => {
            *program_dictionary.borrow_mut() = dictionary_snapshot;
            match plan.rollback(checkpoint) {
                Ok(()) => Err(error),
                Err(rollback_error) => Err(rollback_error),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar_capture_cases() -> Vec<(ValueKind, Value)> {
        let mut cases = Vec::new();
        #[cfg(feature = "u8")]
        cases.push((ValueKind::U8, Value::U8(Ref::new(8))));
        #[cfg(feature = "u16")]
        cases.push((ValueKind::U16, Value::U16(Ref::new(16))));
        #[cfg(feature = "u32")]
        cases.push((ValueKind::U32, Value::U32(Ref::new(32))));
        #[cfg(feature = "u64")]
        cases.push((ValueKind::U64, Value::U64(Ref::new(64))));
        #[cfg(feature = "u128")]
        cases.push((ValueKind::U128, Value::U128(Ref::new(128))));
        #[cfg(feature = "i8")]
        cases.push((ValueKind::I8, Value::I8(Ref::new(-8))));
        #[cfg(feature = "i16")]
        cases.push((ValueKind::I16, Value::I16(Ref::new(-16))));
        #[cfg(feature = "i32")]
        cases.push((ValueKind::I32, Value::I32(Ref::new(-32))));
        #[cfg(feature = "i64")]
        cases.push((ValueKind::I64, Value::I64(Ref::new(-64))));
        #[cfg(feature = "i128")]
        cases.push((ValueKind::I128, Value::I128(Ref::new(-128))));
        #[cfg(feature = "f32")]
        cases.push((ValueKind::F32, Value::F32(Ref::new(3.25))));
        #[cfg(feature = "f64")]
        cases.push((ValueKind::F64, Value::F64(Ref::new(6.5))));
        #[cfg(feature = "complex")]
        cases.push((ValueKind::C64, Value::C64(Ref::new(C64::new(3.0, 4.0)))));
        #[cfg(feature = "rational")]
        cases.push((ValueKind::R64, Value::R64(Ref::new(R64::new(3, 4)))));
        #[cfg(any(feature = "bool", feature = "variable_define"))]
        cases.push((ValueKind::Bool, Value::Bool(Ref::new(true))));
        #[cfg(any(feature = "string", feature = "variable_define"))]
        cases.push((
            ValueKind::String,
            Value::String(Ref::new("captured".to_string())),
        ));
        cases.push((ValueKind::Index, Value::Index(Ref::new(42))));
        #[cfg(feature = "atom")]
        {
            let atom = MechAtom::from_name("captured");
            cases.push((
                ValueKind::Atom(atom.id(), atom.name()),
                Value::Atom(Ref::new(atom)),
            ));
        }
        cases
    }

    #[test]
    fn activation_capture_slot_supports_all_enabled_scalar_kinds() {
        for (kind, source) in scalar_capture_cases() {
            let slot = create_capture_slot_for_kind(&kind).unwrap();
            let cells_before = slot.reactive_root_cell_ids();
            assert_eq!(cells_before.len(), 1);
            commit_capture_slot(&slot, &source).unwrap();
            assert_eq!(slot, source);
            assert_eq!(slot.reactive_root_cell_ids(), cells_before);
        }
    }

    #[cfg(any(feature = "string", feature = "variable_define"))]
    #[test]
    fn activation_capture_slot_preserves_identity_across_updates() {
        let slot = create_capture_slot_for_kind(&ValueKind::String).unwrap();
        let cells = slot.reactive_root_cell_ids();
        commit_capture_slot(&slot, &Value::String(Ref::new("first".to_string()))).unwrap();
        assert_eq!(slot, Value::String(Ref::new("first".to_string())));
        assert_eq!(slot.reactive_root_cell_ids(), cells);
        commit_capture_slot(&slot, &Value::String(Ref::new("second".to_string()))).unwrap();
        assert_eq!(slot, Value::String(Ref::new("second".to_string())));
        assert_eq!(slot.reactive_root_cell_ids(), cells);
    }

    #[cfg(all(feature = "f64", any(feature = "string", feature = "variable_define")))]
    #[test]
    fn activation_capture_slot_rejects_kind_mismatch() {
        let slot = create_capture_slot_for_kind(&ValueKind::F64).unwrap();
        let error =
            commit_capture_slot(&slot, &Value::String(Ref::new("wrong".to_string()))).unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternCaptureKindUnsupported");
    }
}
