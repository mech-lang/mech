use crate::*;

// Functions
// ----------------------------------------------------------------------------

pub fn function_define(fxn_def: &FunctionDefine, p: &Interpreter) -> MResult<FunctionDefinition> {
    let fxn_name_id = fxn_def.name.hash();
    let mut new_fxn =
        FunctionDefinition::new(fxn_name_id, fxn_def.name.to_string(), fxn_def.clone());

    for input_arg in &fxn_def.input {
        new_fxn
            .input
            .insert(input_arg.name.hash(), input_arg.kind.clone());
    }

    for output_arg in &fxn_def.output {
        new_fxn
            .output
            .insert(output_arg.name.hash(), output_arg.kind.clone());
    }

    let functions = p.functions();
    let mut functions_brrw = functions.borrow_mut();
    functions_brrw
        .user_functions
        .insert(fxn_name_id, new_fxn.clone());
    functions_brrw
        .dictionary
        .borrow_mut()
        .insert(fxn_name_id, fxn_def.name.to_string());
    p.state
        .borrow()
        .dictionary
        .borrow_mut()
        .insert(fxn_name_id, fxn_def.name.to_string());

    Ok(new_fxn)
}

pub fn function_call(
    fxn_call: &FunctionCall,
    env: Option<&Environment>,
    p: &Interpreter,
) -> MResult<Value> {
    let functions = p.functions();
    let fxn_name_id = fxn_call.name.hash();

    if let Some(user_fxn) = { functions.borrow().user_functions.get(&fxn_name_id).cloned() } {
        let mut input_arg_values = vec![];
        for (_, arg_expr) in fxn_call.args.iter() {
            input_arg_values.push(expression(arg_expr, env, p)?);
        }
        return execute_user_function(&user_fxn, &input_arg_values, p);
    }

    if { functions.borrow().functions.contains_key(&fxn_name_id) } {
        todo!();
    }

    let fxn_compiler = {
        functions
            .borrow()
            .function_compilers
            .get(&fxn_name_id)
            .copied()
    };
    match fxn_compiler {
        Some(fxn_compiler) => {
            let mut input_arg_values = vec![];
            for (_, arg_expr) in fxn_call.args.iter() {
                input_arg_values.push(expression(arg_expr, env, p)?);
            }
            execute_native_function_compiler(fxn_compiler, &input_arg_values, p)
        }
        None => Err(MechError::new(
            MissingFunctionError {
                function_id: fxn_name_id,
            },
            None,
        )
        .with_compiler_loc()
        .with_tokens(fxn_call.name.tokens())),
    }
}

pub fn register_fsm_implementation(fsm: &FsmImplementation, p: &Interpreter) -> MResult<()> {
    let fsm_id = fsm.name.hash();
    p.user_state_machines
        .borrow_mut()
        .insert(fsm_id, fsm.clone());
    p.state
        .borrow()
        .dictionary
        .borrow_mut()
        .insert(fsm_id, fsm.name.to_string());
    Ok(())
}

pub fn execute_fsm_pipe(
    fsm_pipe: &FsmPipe,
    env: Option<&Environment>,
    p: &Interpreter,
) -> MResult<Value> {
    let fsm_id = fsm_pipe.start.name.hash();
    let fsm = {
        let fsms = p.user_state_machines.borrow();
        fsms.get(&fsm_id).cloned()
    }
    .ok_or_else(|| {
        MechError::new(MissingFunctionError { function_id: fsm_id }, None)
            .with_compiler_loc()
            .with_tokens(fsm_pipe.start.tokens())
    })?;

    let mut call_env = Environment::new();
    let mut args = Vec::new();
    if let Some(start_args) = &fsm_pipe.start.args {
        for (_, arg_expr) in start_args {
            args.push(expression(arg_expr, env, p)?);
        }
    }
    if fsm.input.len() != args.len() {
        return Err(MechError::new(
            IncorrectNumberOfArguments {
                expected: fsm.input.len(),
                found: args.len(),
            },
            None,
        )
        .with_compiler_loc()
        .with_tokens(fsm_pipe.start.tokens()));
    }
    for (arg_name, arg_value) in fsm.input.iter().zip(args.iter()) {
        call_env.insert(arg_name.hash(), detach_value(arg_value));
    }

    let mut state = pattern_to_value(&fsm.start, &call_env, p)?;
    let max_steps = 10_000usize;
    for _ in 0..max_steps {
        let mut transitioned = false;
        for arm in &fsm.arms {
            match arm {
                FsmArm::Transition(pattern, transitions) => {
                    let mut arm_env = call_env.clone();
                    if pattern_matches_value(pattern, &state, &mut arm_env, p)? {
                        let out = apply_transitions(transitions, &mut state, &mut arm_env, p)?;
                        call_env = arm_env;
                        if let Some(value) = out {
                            return Ok(value);
                        }
                        transitioned = true;
                        break;
                    }
                }
                FsmArm::Guard(pattern, guards) => {
                    let mut arm_env = call_env.clone();
                    if !pattern_matches_value(pattern, &state, &mut arm_env, p)? {
                        continue;
                    }
                    for guard in guards {
                        let guard_passes = match &guard.condition {
                            Pattern::Wildcard => true,
                            _ => {
                                let cond = pattern_to_value(&guard.condition, &arm_env, p)?;
                                matches!(cond, Value::Bool(x) if *x.borrow())
                            }
                        };
                        if !guard_passes {
                            continue;
                        }
                        let out =
                            apply_transitions(&guard.transitions, &mut state, &mut arm_env, p)?;
                        call_env = arm_env;
                        if let Some(value) = out {
                            return Ok(value);
                        }
                        transitioned = true;
                        break;
                    }
                    if transitioned {
                        break;
                    }
                }
            }
        }
        if !transitioned {
            return Ok(state);
        }
    }
    Err(MechError::new(FeatureNotEnabledError, Some("FSM exceeded maximum transition limit".to_string())).with_compiler_loc())
}

fn apply_transitions(
    transitions: &[Transition],
    state: &mut Value,
    env: &mut Environment,
    p: &Interpreter,
) -> MResult<Option<Value>> {
    for transition in transitions {
        match transition {
            Transition::Next(next_pattern) | Transition::Async(next_pattern) => {
                *state = pattern_to_value(next_pattern, env, p)?;
            }
            Transition::Output(output_pattern) => {
                return Ok(Some(pattern_to_value(output_pattern, env, p)?));
            }
            Transition::Statement(stmt) => {
                statement(stmt, Some(env), p)?;
            }
            Transition::CodeBlock(code) => {
                for (line, _) in code {
                    mech_code(line, p)?;
                }
            }
        }
    }
    Ok(None)
}

fn pattern_to_value(pattern: &Pattern, env: &Environment, p: &Interpreter) -> MResult<Value> {
    match pattern {
        Pattern::Wildcard => Ok(Value::Empty),
        Pattern::Expression(expr) => expression(expr, Some(env), p),
        Pattern::Tuple(pattern_tuple) => {
            let mut values = Vec::with_capacity(pattern_tuple.0.len());
            for inner in &pattern_tuple.0 {
                values.push(pattern_to_value(inner, env, p)?);
            }
            Ok(Value::Tuple(Ref::new(MechTuple::from_vec(values))))
        }
        Pattern::TupleStruct(pattern_tuple_struct) => {
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
            Ok(Value::Tuple(Ref::new(MechTuple::from_vec(values))))
        }
    }
}

pub fn execute_native_function_compiler(
    fxn_compiler: &'static dyn NativeFunctionCompiler,
    input_arg_values: &Vec<Value>,
    p: &Interpreter,
) -> MResult<Value> {
    let plan = p.plan();
    match fxn_compiler.compile(input_arg_values) {
        Ok(new_fxn) => {
            let mut plan_brrw = plan.borrow_mut();
            new_fxn.solve();
            let result = new_fxn.out();
            plan_brrw.push(new_fxn);
            Ok(result)
        }
        Err(err) => Err(err),
    }
}

fn execute_user_function(
    fxn_def: &FunctionDefinition,
    input_arg_values: &Vec<Value>,
    p: &Interpreter,
) -> MResult<Value> {
    if input_arg_values.len() != fxn_def.input.len() {
        return Err(MechError::new(
            IncorrectNumberOfArguments {
                expected: fxn_def.input.len(),
                found: input_arg_values.len(),
            },
            None,
        )
        .with_compiler_loc()
        .with_tokens(fxn_def.code.name.tokens()));
    }

    let scope = FunctionScope::enter(p);
    bind_function_inputs(fxn_def, input_arg_values, p)?;

    if !fxn_def.code.match_arms.is_empty() {
        let output = execute_function_match_arms(fxn_def, input_arg_values, p);
        drop(scope);
        return output;
    }

    for statement_node in &fxn_def.code.statements {
        statement(statement_node, None, p)?;
    }

    let output = collect_function_output(p, fxn_def);
    drop(scope);
    output
}

fn execute_function_match_arms(
    fxn_def: &FunctionDefinition,
    input_arg_values: &Vec<Value>,
    p: &Interpreter,
) -> MResult<Value> {
    for arm in &fxn_def.code.match_arms {
        let mut env = Environment::new();
        if pattern_matches_arguments(&arm.pattern, input_arg_values, &mut env, p)? {
            let out = expression(&arm.expression, Some(&env), p)?;
            return coerce_function_output_kind(detach_value(&out), fxn_def, p);
        }
    }
    Err(MechError::new(
        FunctionOutputUndefinedError {
            output_id: fxn_def.id,
        },
        None,
    )
    .with_compiler_loc()
    .with_tokens(fxn_def.code.name.tokens()))
}

fn pattern_matches_arguments(
    pattern: &Pattern,
    args: &Vec<Value>,
    env: &mut Environment,
    p: &Interpreter,
) -> MResult<bool> {
    if args.len() == 1 {
        return pattern_matches_value(pattern, &args[0], env, p);
    }
    match pattern {
        Pattern::Tuple(pattern_tuple) => {
            if pattern_tuple.0.len() != args.len() {
                return Ok(false);
            }
            for (pat, arg) in pattern_tuple.0.iter().zip(args.iter()) {
                if !pattern_matches_value(pat, arg, env, p)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn pattern_matches_value(
    pattern: &Pattern,
    value: &Value,
    env: &mut Environment,
    p: &Interpreter,
) -> MResult<bool> {
    match pattern {
        Pattern::Wildcard => Ok(true),
        Pattern::Tuple(pattern_tuple) => match detach_value(value) {
            Value::Tuple(tuple) => {
                let tuple_brrw = tuple.borrow();
                if pattern_tuple.0.len() != tuple_brrw.elements.len() {
                    return Ok(false);
                }
                for (pat, val) in pattern_tuple.0.iter().zip(tuple_brrw.elements.iter()) {
                    if !pattern_matches_value(pat, val, env, p)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        },
        Pattern::Expression(Expression::Var(var)) => {
            let var_id = var.name.hash();
            let detached = detach_value(value);
            if let Some(existing) = env.get(&var_id) {
                Ok(existing == &detached)
            } else {
                env.insert(var_id, detached);
                Ok(true)
            }
        }
        Pattern::Expression(expr) => {
            if let Some(var_id) = extract_pattern_variable_id(expr) {
                let detached = detach_value(value);
                if let Some(existing) = env.get(&var_id) {
                    return Ok(existing == &detached);
                }
                env.insert(var_id, detached);
                return Ok(true);
            }
            let expected = expression(expr, Some(env), p)?;
            Ok(values_match(&detach_value(&expected), &detach_value(value)))
        }
        Pattern::TupleStruct(pat_struct) => match detach_value(value) {
            Value::Tuple(tuple) => {
                let tuple_brrw = tuple.borrow();
                if tuple_brrw.elements.len() != pat_struct.patterns.len() + 1 {
                    return Ok(false);
                }
                let expected_state = atom(
                    &Atom {
                        name: pat_struct.name.clone(),
                    },
                    p,
                );
                if !values_match(&expected_state, &detach_value(&tuple_brrw.elements[0])) {
                    return Ok(false);
                }
                for (pat, val) in pat_struct.patterns.iter().zip(tuple_brrw.elements.iter().skip(1))
                {
                    if !pattern_matches_value(pat, val, env, p)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        },
    }
}

fn extract_pattern_variable_id(expr: &Expression) -> Option<u64> {
    match expr {
        Expression::Var(var) => Some(var.name.hash()),
        Expression::Formula(factor) => match factor {
            Factor::Expression(inner_expr) => extract_pattern_variable_id(inner_expr),
            Factor::Term(term) if term.rhs.is_empty() => extract_pattern_variable_id_from_term(&term.lhs),
            _ => None,
        },
        _ => None,
    }
}

fn extract_pattern_variable_id_from_term(factor: &Factor) -> Option<u64> {
    match factor {
        Factor::Expression(expr) => extract_pattern_variable_id(expr),
        Factor::Parenthetical(inner) => extract_pattern_variable_id_from_term(inner),
        _ => None,
    }
}

fn values_match(expected: &Value, actual: &Value) -> bool {
    if expected == actual {
        return true;
    }
    #[cfg(all(feature = "u64", feature = "f64"))]
    {
        match (expected, actual) {
            (Value::F64(x), Value::U64(y)) => return (*x.borrow() as u64) == *y.borrow(),
            (Value::U64(x), Value::F64(y)) => return *x.borrow() == (*y.borrow() as u64),
            _ => {}
        }
    }
    false
}

fn coerce_function_output_kind(value: Value, fxn_def: &FunctionDefinition, p: &Interpreter) -> MResult<Value> {
    if fxn_def.output.is_empty() {
        return Ok(value);
    }
    let Some((_, output_kind_annotation)) = fxn_def.output.get_index(0) else {
        return Ok(value);
    };
    #[cfg(feature = "kind_annotation")]
    {
    let target_kind = kind_annotation(&output_kind_annotation.kind, p)?
        .to_value_kind(&p.state.borrow().kinds)?;
    Ok(value.convert_to(&target_kind).unwrap_or(value))
    }
    #[cfg(not(feature = "kind_annotation"))]
    {
        let _ = (output_kind_annotation, p);
        Ok(value)
    }
}

struct FunctionScope {
    state: Ref<ProgramState>,
    previous_symbols: SymbolTableRef,
    previous_plan: Plan,
    previous_environment: Option<SymbolTableRef>,
}

impl FunctionScope {
    fn enter(p: &Interpreter) -> Self {
        let state = p.state.clone();
        let mut state_brrw = state.borrow_mut();
        let mut local_symbols = SymbolTable::new();
        local_symbols.dictionary = state_brrw.dictionary.clone();
        let local_symbols = Ref::new(local_symbols);
        let previous_symbols = std::mem::replace(&mut state_brrw.symbol_table, local_symbols);
        let previous_plan = std::mem::replace(&mut state_brrw.plan, Plan::new());
        let previous_environment = state_brrw.environment.take();
        drop(state_brrw);

        Self {
            state,
            previous_symbols,
            previous_plan,
            previous_environment,
        }
    }
}

impl Drop for FunctionScope {
    fn drop(&mut self) {
        let mut state_brrw = self.state.borrow_mut();
        state_brrw.symbol_table = self.previous_symbols.clone();
        state_brrw.plan = self.previous_plan.clone();
        state_brrw.environment = self.previous_environment.clone();
    }
}

fn bind_function_inputs(
    fxn_def: &FunctionDefinition,
    input_arg_values: &Vec<Value>,
    p: &Interpreter,
) -> MResult<()> {
    let scoped_state = p.state.borrow();
    for ((arg_id, _), input_value) in fxn_def.input.iter().zip(input_arg_values.iter()) {
        let arg_name = fxn_def
            .code
            .input
            .iter()
            .find(|arg| arg.name.hash() == *arg_id)
            .map(|arg| arg.name.to_string())
            .unwrap_or_else(|| arg_id.to_string());
        scoped_state.save_symbol(*arg_id, arg_name, detach_value(input_value), false);
    }
    Ok(())
}

fn collect_function_output(p: &Interpreter, fxn_def: &FunctionDefinition) -> MResult<Value> {
    let symbols = p.symbols();
    let symbols_brrw = symbols.borrow();
    let mut outputs = vec![];

    for output_arg in &fxn_def.code.output {
        let output_id = output_arg.name.hash();
        match symbols_brrw.get(output_id) {
            Some(cell) => outputs.push(detach_value(&cell.borrow())),
            None => {
                return Err(
                    MechError::new(FunctionOutputUndefinedError { output_id }, None)
                        .with_compiler_loc()
                        .with_tokens(output_arg.tokens()),
                );
            }
        }
    }

    Ok(match outputs.len() {
        0 => Value::Empty,
        1 => outputs.remove(0),
        _ => Value::Tuple(Ref::new(MechTuple::from_vec(outputs))),
    })
}

pub(crate) fn detach_value(value: &Value) -> Value {
    match value {
        Value::MutableReference(reference) => detach_value(&reference.borrow()),
        _ => value.clone(),
    }
}

#[derive(Debug, Clone)]
pub struct MissingFunctionError {
    pub function_id: u64,
}

impl MechErrorKind for MissingFunctionError {
    fn name(&self) -> &str {
        "MissingFunction"
    }
    fn message(&self) -> String {
        format!("Function with id {} not found", self.function_id)
    }
}

#[derive(Debug, Clone)]
pub struct FunctionOutputUndefinedError {
    pub output_id: u64,
}

impl MechErrorKind for FunctionOutputUndefinedError {
    fn name(&self) -> &str {
        "FunctionOutputUndefined"
    }
    fn message(&self) -> String {
        format!(
            "Function output {} was declared but never defined",
            self.output_id
        )
    }
}
