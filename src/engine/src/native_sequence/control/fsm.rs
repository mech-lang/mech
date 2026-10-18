//! Synchronous typed machines lowered to the existing loop and match IR.
//! The first native slice has one recurring state and one terminal state whose
//! payloads share compile-time typed storage with the recurring state. Completion is private
//! scalar loop state; no nominal state objects or dynamic dispatch are emitted.
use super::*;
use mech_syntax::document::{
    AtomLiteralSyntax, FsmArmBodySyntax, FsmBodyTransitionSyntax, FsmImplementationSyntax,
    FsmPipeSyntax, FsmSpecificationSyntax,
};

#[derive(Clone)]
pub(super) struct Machine {
    inputs: Vec<(String, ControlKind)>,
    implementation_inputs: Vec<String>,
    output: ControlKind,
    states: BTreeMap<String, Vec<ControlKind>>,
    implementation: FsmImplementationSyntax,
}

fn variable_name(variable: &VariableSyntax) -> Result<String, String> {
    match variable.stem().ok_or("native FSM parameter name")? {
        VariableStemSyntax::Identifier(name) => text(name.syntax()),
        _ => Err("native FSM parameters must be local names".into()),
    }
}

pub(super) fn collect(
    syntax: &SyntaxNode,
    aliases: &BTreeMap<String, SyntaxNode>,
) -> Result<BTreeMap<String, Machine>, String> {
    let mut specifications = Vec::new();
    let mut implementations = Vec::new();
    visit(syntax, SyntaxKind::FsmSpecification, &mut specifications);
    visit(syntax, SyntaxKind::FsmImplementation, &mut implementations);
    let mut bodies = BTreeMap::new();
    for node in implementations {
        let body = FsmImplementationSyntax::cast(node).ok_or("FSM implementation")?;
        let name = text(body.name().ok_or("FSM implementation name")?.syntax())?;
        if bodies.insert(name, body).is_some() {
            return Err("duplicate native FSM implementation".into());
        }
    }
    let mut machines = BTreeMap::new();
    for node in specifications {
        let spec = FsmSpecificationSyntax::cast(node).ok_or("FSM specification")?;
        let name = text(spec.name().ok_or("FSM name")?.syntax())?;
        let implementation = bodies
            .remove(&name)
            .ok_or("native FSM requires an implementation")?;
        let mut names = BTreeSet::new();
        let inputs = spec
            .inputs()
            .iter()
            .map(|variable| {
                let name = variable_name(variable)?;
                if !names.insert(name.clone()) {
                    return Err("duplicate native FSM input".into());
                }
                let kind = native_kind(
                    variable
                        .annotation()
                        .ok_or("FSM input kind required")?
                        .syntax(),
                    aliases,
                    &mut BTreeSet::new(),
                )?;
                Ok((name, kind))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let implementation_variables = implementation.inputs();
        if implementation_variables.len() != inputs.len() {
            return Err("native FSM implementation input arity differs from declaration".into());
        }
        names.clear();
        let implementation_inputs = implementation_variables
            .iter()
            .zip(&inputs)
            .map(|(variable, (_, kind))| {
                let name = variable_name(variable)?;
                if !names.insert(name.clone()) {
                    return Err("duplicate native FSM implementation input".into());
                }
                if let Some(annotation) = variable.annotation() {
                    if native_kind(annotation.syntax(), aliases, &mut BTreeSet::new())? != *kind {
                        return Err(
                            "native FSM implementation input kind differs from declaration".into(),
                        );
                    }
                }
                Ok(name)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let output = native_kind(
            spec.output()
                .ok_or("native FSM output kind required")?
                .syntax(),
            aliases,
            &mut BTreeSet::new(),
        )?;
        let mut states = BTreeMap::new();
        for state in spec.states() {
            let state_name = text(state.name().ok_or("FSM state name")?.syntax())?;
            names.clear();
            let kinds = state
                .variables()
                .iter()
                .map(|variable| {
                    if !names.insert(variable_name(variable)?) {
                        return Err("duplicate native FSM state field".into());
                    }
                    native_kind(
                        variable
                            .annotation()
                            .ok_or("FSM state field kind required")?
                            .syntax(),
                        aliases,
                        &mut BTreeSet::new(),
                    )
                })
                .collect::<Result<Vec<_>, String>>()?;
            if states.insert(state_name, kinds).is_some() {
                return Err("duplicate native FSM state".into());
            }
        }
        if states.len() != 2 {
            return Err(
                "native FSM currently requires one recurring state and one terminal state".into(),
            );
        }
        if machines
            .insert(
                name,
                Machine {
                    inputs,
                    implementation_inputs,
                    output,
                    states,
                    implementation,
                },
            )
            .is_some()
        {
            return Err("duplicate native FSM declaration".into());
        }
    }
    if !bodies.is_empty() {
        return Err("native FSM implementation requires a declaration".into());
    }
    Ok(machines)
}

#[derive(Clone)]
struct Arm {
    pattern: PatternSyntax,
    guard: Option<SyntaxNode>,
    transitions: Vec<FsmBodyTransitionSyntax>,
}

fn state(pattern: &PatternSyntax) -> Result<(String, Vec<PatternSyntax>), String> {
    match pattern.value().ok_or("FSM state pattern")? {
        PatternValueSyntax::AtomStruct(value) => Ok((
            text(value.name().ok_or("FSM state name")?.syntax())?,
            value.items(),
        )),
        PatternValueSyntax::Expression(expression) => {
            let mut atoms = Vec::new();
            visit(expression.syntax(), SyntaxKind::AtomLiteral, &mut atoms);
            let atom = atoms
                .first()
                .and_then(|node| AtomLiteralSyntax::cast(node.clone()))
                .ok_or("FSM transition requires a declared state value")?;
            if atoms.len() != 1 || text(expression.syntax())?.trim() != text(atom.syntax())?.trim()
            {
                return Err("FSM transition requires a declared state value".into());
            }
            Ok((text(atom.name().ok_or("FSM atom name")?.syntax())?, vec![]))
        }
        _ => Err("native FSM state requires a named atom and payload".into()),
    }
}

fn arms(machine: &Machine) -> Result<Vec<Arm>, String> {
    let mut out = Vec::new();
    for arm in machine.implementation.arms() {
        match arm.body().ok_or("FSM arm body")? {
            FsmArmBodySyntax::Transition(transition) => out.push(Arm {
                pattern: transition.pattern().ok_or("FSM arm pattern")?,
                guard: None,
                transitions: transition.transitions(),
            }),
            FsmArmBodySyntax::Guard(guarded) => {
                for guard in guarded.guards() {
                    let condition = match guard
                        .condition()
                        .ok_or("FSM guard condition")?
                        .value()
                        .ok_or("FSM guard value")?
                    {
                        PatternValueSyntax::Wildcard(_) => None,
                        PatternValueSyntax::Expression(expression) => {
                            Some(expression.syntax().clone())
                        }
                        _ => {
                            return Err(
                                "native FSM guards require Boolean expressions or a wildcard"
                                    .into(),
                            );
                        }
                    };
                    out.push(Arm {
                        pattern: guarded.pattern().ok_or("FSM arm pattern")?,
                        guard: condition,
                        transitions: guard.transitions(),
                    });
                }
            }
            FsmArmBodySyntax::Comment(_) => {}
        }
    }
    Ok(out)
}

fn boolean(value: bool) -> Expr {
    Expr {
        kind: ControlKind::Boolean,
        op: Op::Boolean(value),
    }
}

fn empty_slot(kind: &ControlKind) -> Result<Expr, String> {
    let op = match kind {
        ControlKind::Integer => Op::Integer(0),
        ControlKind::SignedInteger => Op::SignedInteger(0),
        ControlKind::Float => Op::Float(0),
        ControlKind::Boolean => Op::Boolean(false),
        ControlKind::Bytes(n) => Op::Primitive(
            "sequence/zero-bytes".into(),
            vec![Expr {
                kind: ControlKind::Integer,
                op: Op::Integer(*n as u64),
            }],
        ),
        ControlKind::ReadBytes => Op::ReadBytes(vec![]),
        ControlKind::Floats => Op::Floats(vec![]),
        ControlKind::Tuple(kinds) => {
            Op::Tuple(kinds.iter().map(empty_slot).collect::<Result<_, _>>()?)
        }
        ControlKind::Record(fields) => Op::Tuple(
            fields
                .iter()
                .map(|(_, kind)| empty_slot(kind))
                .collect::<Result<_, _>>()?,
        ),
        _ => return Err("native FSM payload requires unmanaged typed value storage".into()),
    };
    Ok(Expr {
        kind: kind.clone(),
        op,
    })
}

impl Lower<'_> {
    fn fsm_pattern_value(
        &mut self,
        pattern: &PatternSyntax,
        vars: &BTreeMap<String, Expr>,
    ) -> Result<Expr, String> {
        match pattern.value().ok_or("FSM value")? {
            PatternValueSyntax::Expression(expression) => self.expr(expression.syntax(), vars),
            PatternValueSyntax::Tuple(items) => {
                let mut values = items
                    .items()
                    .iter()
                    .map(|p| self.fsm_pattern_value(p, vars))
                    .collect::<Result<Vec<_>, _>>()?;
                if values.len() == 1 {
                    Ok(values.remove(0))
                } else {
                    Ok(tuple(values))
                }
            }
            _ => Err("native FSM payload/output requires an ordinary expression".into()),
        }
    }

    fn fsm_bind(
        &mut self,
        pattern: &PatternSyntax,
        kinds: &[ControlKind],
        values: &[Expr],
    ) -> Result<BTreeMap<String, Expr>, String> {
        let (_, patterns) = state(pattern)?;
        if patterns.len() != kinds.len() {
            return Err("native FSM state pattern payload arity mismatch".into());
        }
        let mut vars = BTreeMap::new();
        let mut bound = BTreeSet::new();
        for ((pattern, kind), value) in patterns.iter().zip(kinds).zip(values) {
            if *kind != value.kind {
                return Err("native FSM state payload kind mismatch".into());
            }
            let (_, total) = self.pattern(pattern, value, &mut vars, &mut bound)?;
            if !total {
                return Err(
                    "native FSM state payloads currently require binding or wildcard patterns"
                        .into(),
                );
            }
        }
        Ok(vars)
    }

    fn fsm_transition(
        &mut self,
        arm: &Arm,
        mut vars: BTreeMap<String, Expr>,
        carry_name: &str,
        done_name: &str,
        machine: &Machine,
        parameters: &[Expr],
        done_slots: &[usize],
    ) -> Result<Expr, String> {
        let mut mutable = BTreeSet::new();
        let mut effects = Vec::new();
        for (i, transition) in arm.transitions.iter().enumerate() {
            match transition {
                FsmBodyTransitionSyntax::Statement(transition) => {
                    let previous = vars.clone();
                    self.branch_statement(
                        transition
                            .statement()
                            .ok_or("FSM transition statement")?
                            .syntax(),
                        &mut vars,
                        &mut mutable,
                    )?;
                    effects.extend(
                        vars.iter()
                            .filter(|(name, value)| previous.get(*name) != Some(*value))
                            .map(|(_, value)| value.clone()),
                    );
                }
                FsmBodyTransitionSyntax::Block(block) => {
                    for item in block.items() {
                        let previous = vars.clone();
                        self.branch_statement(
                            &item.value().ok_or("FSM block statement")?,
                            &mut vars,
                            &mut mutable,
                        )?;
                        effects.extend(
                            vars.iter()
                                .filter(|(name, value)| previous.get(*name) != Some(*value))
                                .map(|(_, value)| value.clone()),
                        );
                    }
                }
                FsmBodyTransitionSyntax::State(transition) => {
                    if i + 1 != arm.transitions.len() {
                        return Err("native FSM state transition must terminate its arm".into());
                    }
                    let (target, payload) = state(
                        &transition
                            .value()
                            .ok_or("FSM target")?
                            .pattern()
                            .ok_or("FSM target value")?,
                    )?;
                    let expected = machine
                        .states
                        .get(&target)
                        .ok_or("native FSM transition targets an undeclared state")?;
                    if payload.len() != expected.len() {
                        return Err("native FSM transition payload arity mismatch".into());
                    }
                    let values = payload
                        .iter()
                        .map(|p| self.fsm_pattern_value(p, &vars))
                        .collect::<Result<Vec<_>, _>>()?;
                    if values.iter().map(|e| e.kind.clone()).collect::<Vec<_>>() != *expected {
                        return Err("native FSM transition payload kind mismatch".into());
                    }
                    let done = target == done_name;
                    if !done && target != carry_name {
                        return Err("unsupported native FSM target".into());
                    }
                    let mut result = parameters.to_vec();
                    for (i, value) in values.into_iter().enumerate() {
                        result[if done { done_slots[i] } else { i }] = value;
                    }
                    result.push(boolean(done));
                    let output = tuple(result);
                    return Ok(if effects.is_empty() {
                        output
                    } else {
                        Expr {
                            kind: output.kind.clone(),
                            op: Op::Sequence {
                                effects,
                                output: Box::new(output),
                            },
                        }
                    });
                }
                FsmBodyTransitionSyntax::Async(_) => {
                    return Err("asynchronous native FSM transitions are unsupported".into());
                }
                FsmBodyTransitionSyntax::Output(_) => {
                    return Err("native FSM output belongs to its terminal state".into());
                }
            }
        }
        Err("native FSM arm requires a terminating state transition".into())
    }

    pub(super) fn machine(
        &mut self,
        pipe: &FsmPipeSyntax,
        vars: &BTreeMap<String, Expr>,
    ) -> Result<Expr, String> {
        if !pipe.stages().is_empty() {
            return Err("native FSM pipe stages are unsupported".into());
        }
        let instance = pipe.instance().ok_or("FSM instance")?;
        let name = text(instance.name().ok_or("FSM invocation name")?.syntax())?;
        let machine = self
            .machines
            .get(&name)
            .cloned()
            .ok_or_else(|| format!("undefined native FSM {name}"))?;
        // Argument expressions belong to the caller, before entering callback
        // scope. Retain their once-only, left-to-right evaluation even if the
        // initial state does not use a particular input.
        let arguments = instance.arguments().ok_or("FSM arguments")?.arguments();
        let values = arguments
            .iter()
            .map(|argument| {
                let AnyCallArgumentSyntax::Positional(argument) = argument else {
                    return Err("native FSM arguments must be positional".into());
                };
                self.expr(argument.value().ok_or("FSM argument value")?.syntax(), vars)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let key = format!("#{name}");
        if !self.active.insert(key.clone()) {
            return Err("recursive native FSM invocation is unsupported".into());
        }
        let result = self.machine_body(values.clone(), &machine).map(|output| {
            if values.is_empty() {
                output
            } else {
                Expr {
                    kind: output.kind.clone(),
                    op: Op::Sequence {
                        effects: values,
                        output: Box::new(output),
                    },
                }
            }
        });
        self.active.remove(&key);
        result.map_err(|e| format!("native FSM {name}: {e}"))
    }

    fn machine_body(&mut self, values: Vec<Expr>, machine: &Machine) -> Result<Expr, String> {
        if values.iter().map(|e| e.kind.clone()).collect::<Vec<_>>()
            != machine
                .inputs
                .iter()
                .map(|(_, k)| k.clone())
                .collect::<Vec<_>>()
        {
            return Err("native FSM input arity/kind mismatch".into());
        }
        let input_vars = machine
            .implementation_inputs
            .iter()
            .cloned()
            .zip(values)
            .collect();
        let (carry_name, initial) = state(
            &machine
                .implementation
                .start()
                .ok_or("FSM start")?
                .pattern()
                .ok_or("FSM start value")?,
        )?;
        let carry_kinds = machine
            .states
            .get(&carry_name)
            .ok_or("native FSM initial state is undeclared")?;
        let (done_name, done_kinds) = machine
            .states
            .iter()
            .find(|(name, _)| **name != carry_name)
            .ok_or("native FSM terminal state")?;
        // State layouts use disjoint typed lanes, reusing compatible lanes
        // across states. Their source field names/order do not constrain each
        // other. This placement is compile-time data, never runtime dispatch.
        let mut layout = carry_kinds.clone();
        let mut used = BTreeSet::new();
        let mut done_slots = Vec::new();
        for kind in done_kinds {
            let slot = layout
                .iter()
                .enumerate()
                .find(|(i, k)| !used.contains(i) && *k == kind)
                .map(|(i, _)| i)
                .unwrap_or_else(|| {
                    layout.push(kind.clone());
                    layout.len() - 1
                });
            used.insert(slot);
            done_slots.push(slot);
        }
        let mut initial = initial
            .iter()
            .map(|p| self.fsm_pattern_value(p, &input_vars))
            .collect::<Result<Vec<_>, _>>()?;
        if initial.iter().map(|e| e.kind.clone()).collect::<Vec<_>>() != *carry_kinds {
            return Err("native FSM initial payload arity/kind mismatch".into());
        }
        initial.extend(
            layout[carry_kinds.len()..]
                .iter()
                .map(empty_slot)
                .collect::<Result<Vec<_>, _>>()?,
        );
        initial.push(boolean(false));
        let initial = tuple(initial);
        let parameters = layout
            .iter()
            .enumerate()
            .map(|(i, kind)| Expr {
                kind: kind.clone(),
                op: Op::Parameter(i),
            })
            .collect::<Vec<_>>();
        let arms = arms(machine)?;
        let mut carry_arms = Vec::new();
        let mut terminal = None;
        let mut exits = false;
        for arm in arms {
            let (state_name, _) = state(&arm.pattern)?;
            if !machine.states.contains_key(&state_name) {
                return Err("native FSM arm names an undeclared state".into());
            }
            if state_name == *done_name {
                if terminal.is_some() || arm.guard.is_some() {
                    return Err(
                        "native FSM terminal state requires one unguarded output arm".into(),
                    );
                }
                if arm.transitions.len() != 1
                    || !matches!(arm.transitions[0], FsmBodyTransitionSyntax::Output(_))
                {
                    return Err("native FSM terminal state requires a direct output".into());
                }
                terminal = Some(arm);
            } else {
                if carry_arms.last().is_some_and(|a: &Arm| a.guard.is_none()) {
                    return Err("unreachable native FSM guarded arm".into());
                }
                for transition in &arm.transitions {
                    if let FsmBodyTransitionSyntax::State(transition) = transition {
                        let (target, _) = state(
                            &transition
                                .value()
                                .ok_or("FSM target")?
                                .pattern()
                                .ok_or("FSM target pattern")?,
                        )?;
                        if !machine.states.contains_key(&target) {
                            return Err("native FSM transition targets an undeclared state".into());
                        }
                        exits |= target == *done_name;
                    }
                    if matches!(transition, FsmBodyTransitionSyntax::Async(_)) {
                        return Err("asynchronous native FSM transitions are unsupported".into());
                    }
                }
                carry_arms.push(arm);
            }
        }
        if !exits {
            return Err("native FSM recurring state requires a terminal-state transition".into());
        }
        // Non-exhaustive guards retain the existing checked Match failure;
        // never invent a transition or execute an unselected branch.
        let mut branches = Vec::new();
        for arm in carry_arms {
            let locals =
                self.fsm_bind(&arm.pattern, carry_kinds, &parameters[..carry_kinds.len()])?;
            let condition = if let Some(guard) = &arm.guard {
                self.expr(guard, &locals)?
            } else {
                boolean(true)
            };
            if condition.kind != ControlKind::Boolean {
                return Err("native FSM guard must be Boolean".into());
            }
            let value = self.fsm_transition(
                &arm,
                locals,
                &carry_name,
                done_name,
                machine,
                &parameters,
                &done_slots,
            )?;
            branches.push((condition, value));
        }
        let step = Expr {
            kind: initial.kind.clone(),
            op: Op::Match {
                id: self.effect_id(),
                scrutinee: Box::new(boolean(true)),
                arms: branches,
            },
        };
        let predicate = self.primitive(
            "==".into(),
            vec![
                Expr {
                    kind: ControlKind::Boolean,
                    op: Op::Parameter(layout.len()),
                },
                boolean(false),
            ],
        )?;
        let mut kinds = layout;
        kinds.push(ControlKind::Boolean);
        let serial = self.effect_id();
        let mut prefix = format!("fsm-internal-{serial}");
        while self
            .source
            .keys()
            .chain(self.functions.keys())
            .any(|name| name.starts_with(&prefix))
        {
            prefix.push_str("-local");
        }
        let predicate_name = format!("{prefix}-predicate");
        let step_name = format!("{prefix}-step");
        self.functions.insert(
            predicate_name.clone(),
            Function {
                inputs: kinds.clone(),
                output: predicate,
                native_inline: true,
            },
        );
        self.functions.insert(
            step_name.clone(),
            Function {
                inputs: kinds,
                output: step,
                native_inline: true,
            },
        );
        let result = Expr {
            kind: initial.kind.clone(),
            op: Op::Loop {
                id: self.effect_id(),
                initial: Box::new(initial),
                predicate: predicate_name,
                step: step_name,
            },
        };
        let arm = terminal.ok_or("native FSM terminal state is not implemented")?;
        let done_values = done_kinds
            .iter()
            .enumerate()
            .map(|(i, kind)| Expr {
                kind: kind.clone(),
                op: Op::Field(Box::new(result.clone()), done_slots[i]),
            })
            .collect::<Vec<_>>();
        let vars = self.fsm_bind(&arm.pattern, done_kinds, &done_values)?;
        let FsmBodyTransitionSyntax::Output(output) = &arm.transitions[0] else {
            unreachable!()
        };
        let output = self.fsm_pattern_value(
            &output
                .value()
                .ok_or("FSM output")?
                .pattern()
                .ok_or("FSM output value")?,
            &vars,
        )?;
        let output = self.coerce(output, &machine.output)?;
        Ok(Expr {
            kind: output.kind.clone(),
            op: Op::Sequence {
                effects: vec![result],
                output: Box::new(output),
            },
        })
    }
}
