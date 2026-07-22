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

    type PlanSnapshot = (
        usize,
        Vec<(
            ReactiveNodeId,
            usize,
            ReactiveNodeKind,
            Vec<u64>,
            Vec<(u64, ReactiveDependencyKind)>,
        )>,
        Vec<(u64, Vec<ReactiveNodeId>)>,
        Vec<(u64, Vec<ReactiveNodeId>)>,
        Vec<PatternActivationRegistration>,
        usize,
    );

    fn interpret(source: &str) -> Interpreter {
        let tree = mech_syntax::parser::parse(source).unwrap();
        let mut interpreter = Interpreter::new_with_full_stdlib(0);
        interpreter.interpret(&tree).unwrap();
        interpreter
    }

    fn interpret_more(interpreter: &mut Interpreter, source: &str) -> MResult<Value> {
        let tree = mech_syntax::parser::parse(source).unwrap();
        interpreter.interpret(&tree)
    }

    fn symbol_ref(interpreter: &Interpreter, name: &str) -> ValRef {
        interpreter
            .symbols()
            .borrow()
            .get(hash_str(name))
            .unwrap_or_else(|| panic!("missing symbol `{name}`"))
    }
    fn symbol(interpreter: &Interpreter, name: &str) -> Value {
        symbol_ref(interpreter, name).borrow().clone()
    }
    fn root_cell(interpreter: &Interpreter, name: &str) -> ReactiveCellId {
        symbol(interpreter, name).reactive_root_cell_ids()[0]
    }
    fn registration(interpreter: &Interpreter) -> PatternActivationRegistration {
        let plan = interpreter.plan();
        let registrations = plan.pattern_activation_registrations();
        assert_eq!(registrations.len(), 1);
        registrations[0].clone()
    }
    fn plan_snapshot(interpreter: &Interpreter) -> PlanSnapshot {
        let plan = interpreter.plan();
        let depth = plan.activation_registration_depth();
        let plan = plan.borrow();
        let nodes = plan
            .nodes
            .iter()
            .map(|node| {
                (
                    node.id,
                    node.plan_index,
                    node.kind,
                    node.outputs.iter().map(|cell| cell.get()).collect(),
                    node.inputs
                        .iter()
                        .map(|dependency| (dependency.cell.get(), dependency.kind))
                        .collect(),
                )
            })
            .collect();
        let mut reactive = plan
            .reactive_consumers
            .iter()
            .map(|(cell, nodes)| (cell.get(), nodes.clone()))
            .collect::<Vec<_>>();
        reactive.sort_by_key(|(cell, _)| *cell);
        let mut sampled = plan
            .sampled_consumers
            .iter()
            .map(|(cell, nodes)| (cell.get(), nodes.clone()))
            .collect::<Vec<_>>();
        sampled.sort_by_key(|(cell, _)| *cell);
        (
            plan.len(),
            nodes,
            reactive,
            sampled,
            plan.pattern_activation_registrations().to_vec(),
            depth,
        )
    }
    fn turn_executed_nodes(outcome: &ReactiveTurnOutcome) -> Vec<ReactiveNodeId> {
        outcome
            .before_commit
            .executed_nodes
            .iter()
            .chain(outcome.after_commit.executed_nodes.iter())
            .copied()
            .collect()
    }
    fn turn_changed_nodes(outcome: &ReactiveTurnOutcome) -> Vec<ReactiveNodeId> {
        outcome
            .before_commit
            .changed_nodes
            .iter()
            .chain(outcome.after_commit.changed_nodes.iter())
            .copied()
            .collect()
    }
    fn turn_unchanged_nodes(outcome: &ReactiveTurnOutcome) -> Vec<ReactiveNodeId> {
        outcome
            .before_commit
            .unchanged_nodes
            .iter()
            .chain(outcome.after_commit.unchanged_nodes.iter())
            .copied()
            .collect()
    }
    fn body_output_f64(interpreter: &Interpreter, arm_index: usize) -> f64 {
        let registration = registration(interpreter);
        let arm = &registration.arms[arm_index];
        let plan = interpreter.plan();
        let plan = plan.borrow();
        for id in (arm.body_node_start..arm.body_node_end).rev() {
            if let Ok(value) = plan.node(id).unwrap().function.out().as_f64() {
                return *value.borrow();
            }
        }
        panic!("no f64 output")
    }
    fn set_enum_event(interpreter: &Interpreter, variant: &str, payload: f64) {
        match symbol(interpreter, "event") {
            Value::Enum(event) => {
                let id = event.borrow().id;
                let names = interpreter
                    .state
                    .borrow()
                    .enums
                    .get(&id)
                    .unwrap()
                    .names
                    .clone();
                *event.borrow_mut() = MechEnum {
                    id,
                    variants: vec![(hash_str(variant), Some(Value::F64(Ref::new(payload))))],
                    names,
                };
            }
            Value::Tuple(_) => set_atom_tuple_event(interpreter, variant, payload),
            _ => panic!("event is neither enum nor tagged tuple"),
        }
    }
    fn set_atom_tuple_event(interpreter: &Interpreter, tag: &str, payload: f64) {
        let Value::Tuple(event) = symbol(interpreter, "event") else {
            panic!("event is not tuple")
        };
        *event.borrow_mut() = MechTuple::from_vec(vec![
            Value::Atom(Ref::new(MechAtom::from_name(tag))),
            Value::F64(Ref::new(payload)),
        ]);
    }
    fn set_tuple_event(interpreter: &Interpreter, values: Vec<Value>) {
        let Value::Tuple(event) = symbol(interpreter, "event") else {
            panic!("event is not tuple")
        };
        *event.borrow_mut() = MechTuple::from_vec(values);
    }
    fn selected_arm_index(
        registration: &PatternActivationRegistration,
        outcome: &ReactiveTurnOutcome,
    ) -> usize {
        let changed = turn_changed_nodes(outcome);
        registration
            .arms
            .iter()
            .position(|arm| changed.contains(&arm.gate_node))
            .expect("no selected gate")
    }
    fn assert_dispatch_turn(
        interpreter: &Interpreter,
        topology: &PlanSnapshot,
        outcome: &ReactiveTurnOutcome,
        expected_arm: usize,
        output: f64,
    ) {
        let registration = registration(interpreter);
        let executed = turn_executed_nodes(outcome);
        let changed = turn_changed_nodes(outcome);
        let unchanged = turn_unchanged_nodes(outcome);
        assert_eq!(
            executed
                .iter()
                .filter(|id| **id == registration.scope_pulse_node)
                .count(),
            1
        );
        assert_eq!(
            executed
                .iter()
                .filter(|id| **id == registration.selector_node)
                .count(),
            1
        );
        assert_eq!(selected_arm_index(&registration, outcome), expected_arm);
        for (index, arm) in registration.arms.iter().enumerate() {
            for node in [arm.matcher_node, arm.finalizer_node, arm.gate_node] {
                assert_eq!(executed.iter().filter(|id| **id == node).count(), 1);
            }
            if index == expected_arm {
                assert!(changed.contains(&arm.gate_node));
                assert!(!unchanged.contains(&arm.gate_node));
                for node in arm.body_node_start..arm.body_node_end {
                    assert_eq!(executed.iter().filter(|id| **id == node).count(), 1);
                }
            } else {
                assert!(unchanged.contains(&arm.gate_node));
                assert!(!changed.contains(&arm.gate_node));
                for node in arm.body_node_start..arm.body_node_end {
                    assert!(!executed.contains(&node));
                }
            }
        }
        assert_eq!(body_output_f64(interpreter, expected_arm), output);
        assert_eq!(&plan_snapshot(interpreter), topology);
    }

    const ENUM_ACTIVATION: &str = r#"event := (:pressed, 0.0)

~> event
  | :pressed(x) => {
      selected := x + 0.0
    }
  | :released(x) => {
      selected := x + 1000.0
    }
  | * => {
      selected := -1.0
    }
"#;
    fn load_enum_activation() -> (
        Interpreter,
        ReactiveCellId,
        PatternActivationRegistration,
        PlanSnapshot,
    ) {
        let interpreter = interpret(ENUM_ACTIVATION);
        let trigger = root_cell(&interpreter, "event");
        let registration = registration(&interpreter);
        assert_eq!(registration.arms.len(), 3);
        assert_eq!(registration.arms[0].captures.len(), 1);
        assert_eq!(registration.arms[1].captures.len(), 1);
        assert!(registration.arms[2].captures.is_empty());
        assert!(!interpreter.symbols().borrow().contains(hash_str("x")));
        assert!(
            !interpreter
                .symbols()
                .borrow()
                .contains(hash_str("selected"))
        );
        let topology = plan_snapshot(&interpreter);
        (interpreter, trigger, registration, topology)
    }

    #[test]
    fn activation_pattern_selects_pressed_released_and_wildcard() {
        let (mut i, trigger, _, topology) = load_enum_activation();
        for (name, payload, arm, output) in [
            ("pressed", 10., 0, 10.),
            ("released", 20., 1, 1020.),
            ("other", 30., 2, -1.),
        ] {
            set_enum_event(&i, name, payload);
            let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
            assert_dispatch_turn(&i, &topology, &outcome, arm, output);
        }
    }
    #[test]
    fn activation_pattern_enum_arms_compile_independent_of_initial_variant() {
        let (mut i, trigger, r, topology) = load_enum_activation();
        assert_eq!(r.arms[1].captures[0].kind, ValueKind::F64);
        set_enum_event(&i, "released", 20.);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 1, 1020.);
    }
    #[test]
    fn activation_pattern_enum_payload_capture_is_available() {
        let (mut i, trigger, r, topology) = load_enum_activation();
        let cell = r.arms[0].captures[0].cell;
        assert!(
            i.plan().borrow().nodes[r.arms[0].body_node_start..r.arms[0].body_node_end]
                .iter()
                .any(|n| n.inputs.iter().any(|d| d.cell == cell))
        );
        set_enum_event(&i, "pressed", 10.);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 10.);
    }
    #[test]
    fn activation_pattern_equal_packets_dispatch_repeatedly() {
        let (mut i, trigger, _, topology) = load_enum_activation();
        set_enum_event(&i, "pressed", 30.);
        for _ in 0..2 {
            let o = i.advance_reactive_turn(&[trigger]).unwrap();
            assert_dispatch_turn(&i, &topology, &o, 0, 30.);
        }
    }
    #[test]
    fn activation_pattern_unselected_arm_nodes_do_not_execute() {
        let (mut i, trigger, r, topology) = load_enum_activation();
        set_enum_event(&i, "released", 20.);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 1, 1020.);
        let executed = turn_executed_nodes(&o);
        for arm in [&r.arms[0], &r.arms[2]] {
            for node in arm.body_node_start..arm.body_node_end {
                assert!(!executed.contains(&node));
            }
        }
    }
    #[test]
    fn activation_pattern_switching_arms_does_not_grow_plan() {
        let (mut i, trigger, _, topology) = load_enum_activation();
        for (name, payload) in [
            ("pressed", 10.),
            ("released", 20.),
            ("other", 30.),
            ("pressed", 30.),
            ("pressed", 30.),
        ] {
            set_enum_event(&i, name, payload);
            i.advance_reactive_turn(&[trigger]).unwrap();
            assert_eq!(plan_snapshot(&i), topology);
        }
    }
    #[test]
    fn activation_pattern_capture_storage_identity_is_stable() {
        let (mut i, trigger, r, topology) = load_enum_activation();
        let captures = r
            .arms
            .iter()
            .flat_map(|arm| arm.captures.iter())
            .map(|capture| (capture.id, capture.kind.clone(), capture.cell))
            .collect::<Vec<_>>();
        for (name, payload) in [("pressed", 10.), ("released", 20.), ("other", 30.)] {
            set_enum_event(&i, name, payload);
            i.advance_reactive_turn(&[trigger]).unwrap();
            let current = registration(&i)
                .arms
                .iter()
                .flat_map(|arm| arm.captures.iter())
                .map(|capture| (capture.id, capture.kind.clone(), capture.cell))
                .collect::<Vec<_>>();
            assert_eq!(current, captures);
            assert_eq!(plan_snapshot(&i), topology);
        }
    }

    const ATOM_TUPLE_ACTIVATION: &str = r#"
event := (:pressed, 0.0)
~> event
  | :pressed(x) => {
      selected := x + 0.0
    }
  | :released(x) => {
      selected := x + 1000.0
    }
  | * => {
      selected := -1.0
    }
"#;
    fn load_atom_tuple_activation() -> (
        Interpreter,
        ReactiveCellId,
        PatternActivationRegistration,
        PlanSnapshot,
    ) {
        let i = interpret(ATOM_TUPLE_ACTIVATION);
        let trigger = root_cell(&i, "event");
        let r = registration(&i);
        let topology = plan_snapshot(&i);
        (i, trigger, r, topology)
    }
    #[test]
    fn activation_pattern_atom_tagged_tuple_selects_arm() {
        let (mut i, trigger, _, topology) = load_atom_tuple_activation();
        for (tag, payload, arm, output) in [
            ("pressed", 10., 0, 10.),
            ("released", 20., 1, 1020.),
            ("other", 30., 2, -1.),
        ] {
            set_atom_tuple_event(&i, tag, payload);
            let o = i.advance_reactive_turn(&[trigger]).unwrap();
            assert_dispatch_turn(&i, &topology, &o, arm, output);
        }
    }
    #[test]
    fn activation_pattern_atom_tagged_tuple_captures_payload() {
        let (mut i, trigger, r, topology) = load_atom_tuple_activation();
        assert_eq!(r.arms[0].captures[0].kind, ValueKind::F64);
        let cell = r.arms[0].captures[0].cell;
        assert!(
            i.plan().borrow().nodes[r.arms[0].body_node_start..r.arms[0].body_node_end]
                .iter()
                .any(|n| n.inputs.iter().any(|d| d.cell == cell))
        );
        set_atom_tuple_event(&i, "pressed", 10.);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 10.);
    }
    #[test]
    fn activation_pattern_atom_tuple_arms_compile_independent_of_initial_tag() {
        let (mut i, trigger, r, topology) = load_atom_tuple_activation();
        assert_eq!(r.arms[0].captures[0].kind, ValueKind::F64);
        assert_eq!(r.arms[1].captures[0].kind, ValueKind::F64);
        set_atom_tuple_event(&i, "released", 20.);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 1, 1020.);
    }

    const FLAT_TUPLE_ACTIVATION: &str = r#"
event := (1.0, 2.0)
~> event
  | (x, y) => {
      selected := x * 10.0 + y
    }
  | * => {
      selected := -1.0
    }
"#;
    const NESTED_TUPLE_ACTIVATION: &str = r#"
event := ((1.0, 2.0), 3.0)
~> event
  | ((x, y), z) => {
      selected := x * 100.0 + y * 10.0 + z
    }
  | * => {
      selected := -1.0
    }
"#;
    const REPEATED_CAPTURE_ACTIVATION: &str = r#"
event := (1.0, 1.0)
~> event
  | (x, x) => {
      selected := x
    }
  | * => {
      selected := -1.0
    }
"#;
    fn tuple_fixture(source: &str) -> (Interpreter, ReactiveCellId, PlanSnapshot) {
        let i = interpret(source);
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);
        (i, trigger, topology)
    }
    #[test]
    fn activation_pattern_tuple_captures_elements() {
        let (mut i, trigger, topology) = tuple_fixture(FLAT_TUPLE_ACTIVATION);
        set_tuple_event(&i, vec![Value::F64(Ref::new(3.)), Value::F64(Ref::new(4.))]);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 34.);
    }
    #[test]
    fn activation_pattern_nested_tuple_captures_elements() {
        let (mut i, trigger, topology) = tuple_fixture(NESTED_TUPLE_ACTIVATION);
        set_tuple_event(
            &i,
            vec![
                Value::Tuple(Ref::new(MechTuple::from_vec(vec![
                    Value::F64(Ref::new(4.)),
                    Value::F64(Ref::new(5.)),
                ]))),
                Value::F64(Ref::new(6.)),
            ],
        );
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 456.);
    }
    #[test]
    fn activation_pattern_repeated_capture_requires_equal_values() {
        let (mut i, trigger, topology) = tuple_fixture(REPEATED_CAPTURE_ACTIVATION);
        set_tuple_event(&i, vec![Value::F64(Ref::new(2.)), Value::F64(Ref::new(2.))]);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 2.);
        set_tuple_event(&i, vec![Value::F64(Ref::new(2.)), Value::F64(Ref::new(3.))]);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 1, -1.);
    }
    #[test]
    fn activation_pattern_repeated_capture_kind_mismatch_is_structured() {
        let mut i = interpret("event := (1.0, \"one\")");
        let topology = plan_snapshot(&i);
        let error = interpret_more(
            &mut i,
            "~> event\n  | (x, x) => {
      selected := x
    }\n  | * => {
      selected := 0.0
    }",
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternCaptureKindUnsupported");
        assert_eq!(plan_snapshot(&i), topology);
        assert!(!i.symbols().borrow().contains(hash_str("x")));
        assert!(!i.symbols().borrow().contains(hash_str("selected")));
    }

    #[test]
    fn activation_pattern_capture_does_not_leak() {
        let (mut i, trigger, topology) = tuple_fixture(FLAT_TUPLE_ACTIVATION);
        for name in ["x", "y", "selected"] {
            assert!(!i.symbols().borrow().contains(hash_str(name)));
        }
        set_tuple_event(&i, vec![Value::F64(Ref::new(3.)), Value::F64(Ref::new(4.))]);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 34.);
    }
    #[test]
    fn activation_pattern_capture_shadows_and_restores_outer_symbol() {
        let mut i = interpret("event := (1.0, 2.0)\nx := 99.0");
        let outer = symbol_ref(&i, "x");
        let address = outer.addr();
        interpret_more(
            &mut i,
            "~> event\n  | (x, y) => {
      selected := x + y
    }\n  | * => {
      selected := -1.0
    }",
        )
        .unwrap();
        assert_eq!(*symbol(&i, "x").as_f64().unwrap().borrow(), 99.);
        assert_eq!(symbol_ref(&i, "x").addr(), address);
        assert!(!i.symbols().borrow().contains(hash_str("y")));
        assert!(!i.symbols().borrow().contains(hash_str("selected")));
        let topology = plan_snapshot(&i);
        let trigger = root_cell(&i, "event");
        set_tuple_event(&i, vec![Value::F64(Ref::new(3.)), Value::F64(Ref::new(4.))]);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 7.);
    }
    #[test]
    fn activation_pattern_arm_definitions_do_not_leak_between_arms() {
        let mut i = interpret("event := (1.0, 2.0)");
        let symbols = i.symbols().borrow().snapshot();
        let dictionary = i.dictionary().borrow().clone();
        let topology = plan_snapshot(&i);
        let error = interpret_more(
            &mut i,
            "~> event\n  | (x, y) => {
      first-local := x + y
    }\n  | (a, b) => {
      second-local := first-local + a + b
    }\n  | * => {
      fallback := 0.0
    }",
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "UndefinedVariable");
        assert_eq!(i.symbols().borrow().snapshot(), symbols);
        assert_eq!(*i.dictionary().borrow(), dictionary);
        assert_eq!(plan_snapshot(&i), topology);
        for name in [
            "first-local",
            "second-local",
            "fallback",
            "x",
            "y",
            "a",
            "b",
        ] {
            assert!(!i.symbols().borrow().contains(hash_str(name)));
        }
    }

    fn failed_elaboration_fixture() -> (
        Interpreter,
        SymbolTableSnapshot,
        Dictionary,
        PlanSnapshot,
        ValRef,
        usize,
    ) {
        let i = interpret("event := (1.0, 2.0)\nouter := 99.0");
        let symbols = i.symbols().borrow().snapshot();
        let dictionary = i.dictionary().borrow().clone();
        let topology = plan_snapshot(&i);
        let outer = symbol_ref(&i, "outer");
        let address = outer.addr();
        (i, symbols, dictionary, topology, outer, address)
    }
    fn assert_failed_elaboration_restored() -> (
        Interpreter,
        SymbolTableSnapshot,
        Dictionary,
        PlanSnapshot,
        usize,
    ) {
        let (mut i, symbols, dictionary, topology, outer, address) = failed_elaboration_fixture();
        let error=interpret_more(&mut i,"~> event\n  | (x, y) => {\n      local-atom := :temporary\n      local-first := x + y\n      local-failure := function-that-does-not-exist(local-first)\n    }\n  | * => {
      fallback := 0.0
    }").unwrap_err();
        assert!(error.kind_name().contains("Function"));
        assert!(!i.dictionary().borrow().contains_key(&hash_str("temporary")));
        for name in [
            "local-atom",
            "local-first",
            "local-failure",
            "fallback",
            "x",
            "y",
        ] {
            assert!(!i.symbols().borrow().contains(hash_str(name)));
        }
        assert_eq!(*symbol(&i, "outer").as_f64().unwrap().borrow(), 99.);
        assert_eq!(symbol_ref(&i, "outer").addr(), address);
        drop(outer);
        (i, symbols, dictionary, topology, address)
    }
    #[test]
    fn activation_pattern_elaboration_error_restores_symbol_table() {
        let (i, symbols, _, _, _) = assert_failed_elaboration_restored();
        assert_eq!(i.symbols().borrow().snapshot(), symbols);
    }
    #[test]
    fn activation_pattern_elaboration_error_restores_program_dictionary() {
        let (i, _, dictionary, _, _) = assert_failed_elaboration_restored();
        assert_eq!(*i.dictionary().borrow(), dictionary);
    }
    #[test]
    fn activation_pattern_elaboration_error_restores_plan() {
        let (i, _, _, topology, _) = assert_failed_elaboration_restored();
        assert_eq!(plan_snapshot(&i), topology);
    }
    #[test]
    fn activation_pattern_preflight_error_does_not_modify_plan() {
        let mut i = interpret("event := (1.0, \"one\")");
        let topology = plan_snapshot(&i);
        let error = interpret_more(
            &mut i,
            "~> event\n  | (x, x) => {
      selected := x
    }\n  | * => {
      selected := 0.0
    }",
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternCaptureKindUnsupported");
        assert_eq!(plan_snapshot(&i), topology);
    }
    #[test]
    fn activation_pattern_recursive_preflight_rejects_nested_activation() {
        let mut i = interpret("event := 1.0\ntick := 0.0");
        let symbols = i.symbols().borrow().snapshot();
        let dictionary = i.dictionary().borrow().clone();
        let topology = plan_snapshot(&i);
        let error = interpret_more(
            &mut i,
            "~> event\n  | 1.0 => {\n      ~> tick {\n        nested := 1.0\n      }\n    }\n  | * => {\n      fallback := 0.0\n    }",
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternDefinitionUnsupported");
        assert_eq!(i.symbols().borrow().snapshot(), symbols);
        assert_eq!(*i.dictionary().borrow(), dictionary);
        assert_eq!(plan_snapshot(&i), topology);
        assert!(!i.symbols().borrow().contains(hash_str("nested")));
        assert!(!i.symbols().borrow().contains(hash_str("fallback")));
    }
    #[test]
    fn activation_pattern_recursive_preflight_rejects_context_declaration() {
        let mut i = interpret("event := 1.0");
        let symbols = i.symbols().borrow().snapshot();
        let dictionary = i.dictionary().borrow().clone();
        let topology = plan_snapshot(&i);
        let error = interpret_more(
            &mut i,
            "~> event\n  | 1.0 => {
      @temporary := test://resource
    }\n  | * => {
      fallback := 0.0
    }",
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternDefinitionUnsupported");
        assert_eq!(i.symbols().borrow().snapshot(), symbols);
        assert_eq!(*i.dictionary().borrow(), dictionary);
        assert_eq!(plan_snapshot(&i), topology);
        assert!(!i.symbols().borrow().contains(hash_str("fallback")));
    }
}
