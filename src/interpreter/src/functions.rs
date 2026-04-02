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
    if p.trace {
        return function_call_traced(fxn_call, env, p);
    }
    function_call_untraced(fxn_call, env, p)
}

fn function_call_untraced(
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

fn function_call_traced(
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
            println!(
                "{}",
                format_trace(
                    "fn",
                    format!(
                        "native {}({})",
                        fxn_call.name.to_string(),
                        format_trace_args(&input_arg_values)
                    ),
                )
            );
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

pub fn execute_native_function_compiler(
    fxn_compiler: &'static dyn NativeFunctionCompiler,
    input_arg_values: &Vec<Value>,
    p: &Interpreter,
) -> MResult<Value> {
    if p.trace {
        return execute_native_function_compiler_traced(fxn_compiler, input_arg_values, p);
    }
    execute_native_function_compiler_untraced(fxn_compiler, input_arg_values, p)
}

fn execute_native_function_compiler_untraced(
    fxn_compiler: &'static dyn NativeFunctionCompiler,
    input_arg_values: &Vec<Value>,
    p: &Interpreter,
) -> MResult<Value> {
    let plan = p.plan();
    match fxn_compiler.compile(input_arg_values) {
        Ok(mut new_fxn) => {
            let mut plan_brrw = plan.borrow_mut();
            new_fxn.solve();
            let result = new_fxn.out();
            plan_brrw.push(new_fxn);
            Ok(result)
        }
        Err(err) => Err(err),
    }
}

fn execute_native_function_compiler_traced(
    fxn_compiler: &'static dyn NativeFunctionCompiler,
    input_arg_values: &Vec<Value>,
    p: &Interpreter,
) -> MResult<Value> {
    let plan = p.plan();
    match fxn_compiler.compile(input_arg_values) {
        Ok(mut new_fxn) => {
            let arm_name = new_fxn
                .to_string()
                .lines()
                .next()
                .unwrap_or("<unknown-arm>")
                .to_string();
            println!(
                "{}",
                format_trace(
                    "arm",
                    format!(
                        "selected {} args=[{}]",
                        arm_name,
                        format_trace_args(input_arg_values)
                    ),
                )
            );
            let mut plan_brrw = plan.borrow_mut();
            new_fxn.solve();
            let result = new_fxn.out();
            println!(
                "{}",
                format_trace("arm", format!("result {}", summarize_value(&result)))
            );
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
    if p.trace {
        return execute_user_function_traced(fxn_def, input_arg_values, p);
    }
    execute_user_function_untraced(fxn_def, input_arg_values, p)
}

fn execute_user_function_untraced(
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

fn execute_user_function_traced(
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

    println!(
        "{}",
        format_trace(
            "fn",
            format!(
                "enter {}({})",
                fxn_def.name,
                format_trace_args(input_arg_values)
            ),
        )
    );

    let scope = FunctionScope::enter(p);
    bind_function_inputs(fxn_def, input_arg_values, p)?;

    let output = if !fxn_def.code.match_arms.is_empty() {
        execute_function_match_arms_traced(fxn_def, input_arg_values, p)
    } else {
        for statement_node in &fxn_def.code.statements {
            statement(statement_node, None, p)?;
        }
        collect_function_output(p, fxn_def)
    };

    drop(scope);

    match output {
        Ok(value) => {
            println!(
                "{}",
                format_trace(
                    "fn",
                    format!("exit  {} => {}", fxn_def.name, summarize_value(&value))
                )
            );
            Ok(value)
        }
        Err(err) => {
            println!(
                "{}",
                format_trace("fn", format!("fail  {} => {:?}", fxn_def.name, err))
            );
            Err(err)
        }
    }
}

fn execute_function_match_arms(
    fxn_def: &FunctionDefinition,
    input_arg_values: &Vec<Value>,
    p: &Interpreter,
) -> MResult<Value> {
    if p.trace {
        return execute_function_match_arms_traced(fxn_def, input_arg_values, p);
    }
    execute_function_match_arms_untraced(fxn_def, input_arg_values, p)
}

fn execute_function_match_arms_untraced(
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

fn execute_function_match_arms_traced(
    fxn_def: &FunctionDefinition,
    input_arg_values: &Vec<Value>,
    p: &Interpreter,
) -> MResult<Value> {
    for (arm_idx, arm) in fxn_def.code.match_arms.iter().enumerate() {
        let mut env = Environment::new();
        let args_summary = summarize_values_with_kinds(input_arg_values);
        let pattern_summary = summarize_pattern(&arm.pattern);
        let matched = pattern_matches_arguments(&arm.pattern, input_arg_values, &mut env, p)?;
        let marker = if matched { "✓" } else { "X" };
        println!(
            "{}",
            format_trace(
                "match",
                format!(
                    "arm[{arm_idx}] test pattern={pattern_summary} args=[{args_summary}] {marker}"
                )
            )
        );
        if matched {
            let out = expression(&arm.expression, Some(&env), p)?;
            let coerced = coerce_function_output_kind(detach_value(&out), fxn_def, p)?;
            println!(
                "{}",
                format_trace(
                    "match",
                    format!(
                        "arm[{arm_idx}] out  value={} kind={}",
                        summarize_value(&coerced),
                        coerced.kind().to_string()
                    )
                )
            );
            return Ok(coerced);
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

fn format_trace(scope: &str, message: String) -> String {
    format!("[trace][{scope}] {message}")
}

fn format_trace_args(values: &Vec<Value>) -> String {
    values
        .iter()
        .map(summarize_value)
        .collect::<Vec<_>>()
        .join(", ")
}

fn summarize_value(value: &Value) -> String {
    const MAX_TRACE_CHARS: usize = 96;
    let rendered = single_line_trace_text(&format!("{:?}", value));
    truncate_for_trace(&rendered, MAX_TRACE_CHARS)
}

fn summarize_values_with_kinds(values: &Vec<Value>) -> String {
    values
        .iter()
        .enumerate()
        .map(|(idx, value)| {
            format!(
                "#{idx}={} :{}",
                summarize_value(value),
                value.kind().to_string()
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn summarize_pattern(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Wildcard => "_".to_string(),
        Pattern::Expression(expr) => truncate_for_trace(&format!("{:?}", expr), 72),
        Pattern::Tuple(tuple) => format!("tuple(len={})", tuple.0.len()),
        Pattern::TupleStruct(tuple_struct) => {
            format!(
                "{}(len={})",
                tuple_struct.name.to_string(),
                tuple_struct.patterns.len()
            )
        }
    }
}

fn truncate_for_trace(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut truncated = text.chars().take(max_chars).collect::<String>();
    truncated.push('…');
    truncated
}

fn single_line_trace_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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

pub(crate) fn pattern_matches_value(
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
                for (pat, val) in pat_struct
                    .patterns
                    .iter()
                    .zip(tuple_brrw.elements.iter().skip(1))
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
            Factor::Term(term) if term.rhs.is_empty() => {
                extract_pattern_variable_id_from_term(&term.lhs)
            }
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

fn coerce_function_output_kind(
    value: Value,
    fxn_def: &FunctionDefinition,
    p: &Interpreter,
) -> MResult<Value> {
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
