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
        #[cfg(feature = "f64")]
        ValueKind::F64 => Ok(Value::F64(Ref::new(0.0))),
        #[cfg(feature = "f32")]
        ValueKind::F32 => Ok(Value::F32(Ref::new(0.0))),
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
    for a in arms {
        if let ActivationArmBody::Block(body) = &a.body {
            for (code, _) in body {
                match code {
                    MechCode::Statement(Statement::VariableAssign(_))
                    | MechCode::Statement(Statement::OpAssign(_)) => {
                        return Err(MechError::new(
                            ActivationPatternRegisterWriteUnsupported,
                            None,
                        )
                        .with_tokens(code.tokens()));
                    }
                    MechCode::Statement(Statement::ContextSend(_)) => {
                        return Err(MechError::new(
                            ActivationPatternContextEffectUnsupported,
                            None,
                        )
                        .with_tokens(code.tokens()));
                    }
                    MechCode::Statement(Statement::VariableDefine(d)) if d.mutable => {
                        return Err(MechError::new(ActivationPatternDefinitionUnsupported, None)
                            .with_tokens(code.tokens()));
                    }
                    _ => {}
                }
            }
        }
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

fn elaborate_patterned_activation_inner(
    scope: &ActivationScope,
    arms: &[ActivationArm],
    trigger: Value,
    trigger_cells: Vec<ReactiveCellId>,
    i: &Interpreter,
) -> MResult<Value> {
    let preflight = preflight_patterned_activation(scope, arms, &trigger, &trigger_cells, i)?;
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
        matched.push(f)
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
        done.push(v)
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
        pulses.push(v)
    }
    let mut ranges = Vec::new();
    for (arm, compiled_arm) in arms.iter().zip(compiled.iter()) {
        let symbols = i.symbols();
        let snapshot = symbols.borrow().snapshot();
        {
            let mut s = symbols.borrow_mut();
            for c in &compiled_arm.captures {
                s.insert(c.id, c.slot.clone(), false);
                s.dictionary.borrow_mut().insert(c.id, c.name.clone());
            }
        }
        let start = plan.len();
        plan.push_activation_registration_scope(pulses[ranges.len()].reactive_root_cell_ids());
        let result = match &arm.body {
            ActivationArmBody::Block(b) => {
                for (c, _) in b {
                    crate::mech_code(c, i)?;
                }
                Ok(())
            }
            ActivationArmBody::Expression(e) => crate::expression(e, None, i).map(|_| ()),
        };
        plan.pop_activation_registration_scope();
        symbols.borrow_mut().restore(snapshot);
        result?;
        ranges.push((start, plan.len()))
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
    let plan = interpreter.plan();
    let checkpoint = plan.checkpoint();
    match elaborate_patterned_activation_inner(scope, arms, trigger, trigger_cells, interpreter) {
        Ok(value) => Ok(value),
        Err(error) => {
            plan.rollback(checkpoint)?;
            Err(error)
        }
    }
}
