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
            trace_println!(
                p,
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
    let plan = p.plan();
    match fxn_compiler.compile(input_arg_values) {
        Ok(mut new_fxn) => {
            trace_println!(
                p,
                "{}",
                format_trace(
                    "arm",
                    format!(
                        "selected {} args=[{}]",
                        new_fxn
                            .to_string()
                            .lines()
                            .next()
                            .unwrap_or("<unknown-arm>"),
                        format_trace_args(input_arg_values)
                    ),
                )
            );
            let mut plan_brrw = plan.borrow_mut();
            new_fxn.solve();
            let result = new_fxn.out();
            trace_println!(
                p,
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

    trace_println!(
        p,
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
        execute_function_match_arms(fxn_def, input_arg_values, p)
    } else {
        for statement_node in &fxn_def.code.statements {
            statement(statement_node, None, p)?;
        }
        collect_function_output(p, fxn_def)
    };

    drop(scope);

    match output {
        Ok(value) => {
            trace_println!(
                p,
                "{}",
                format_trace(
                    "fn",
                    format!("exit  {} => {}", fxn_def.name, summarize_value(&value))
                )
            );
            Ok(value)
        }
        Err(err) => {
            trace_println!(
                p,
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
    for (arm_idx, arm) in fxn_def.code.match_arms.iter().enumerate() {
        let mut env = Environment::new();
        let matched = pattern_matches_arguments(&arm.pattern, input_arg_values, &mut env, p)?;
        trace_println!(p, "{}", {
            let args_summary = summarize_values_with_kinds(input_arg_values);
            let pattern_summary = summarize_pattern(&arm.pattern);
            let marker = if matched { "✓" } else { "X" };
            format_trace(
                "match",
                format!(
                    "arm[{arm_idx}] test pattern={pattern_summary} args=[{args_summary}] {marker}"
                ),
            )
        });
        if matched {
            let out = expression(&arm.expression, Some(&env), p)?;
            let coerced = coerce_function_output_kind(detach_value(&out), fxn_def, p)?;
            trace_println!(
                p,
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
    let rendered = single_line_trace_text(&summarize_value_compact(value, 0));
    truncate_for_trace(&rendered, MAX_TRACE_CHARS)
}

fn summarize_value_compact(value: &Value, depth: usize) -> String {
    if depth > 2 {
        return format!("{}(..)", value.kind().to_string());
    }
    match value {
        #[cfg(feature = "u64")]
        Value::U64(x) => format!("u64(@{:04x}:{})", short_addr(x.addr()), *x.borrow()),
        #[cfg(feature = "i64")]
        Value::I64(x) => format!("i64(@{:04x}:{})", short_addr(x.addr()), *x.borrow()),
        #[cfg(feature = "f64")]
        Value::F64(x) => format!("f64(@{:04x}:{})", short_addr(x.addr()), *x.borrow()),
        #[cfg(feature = "bool")]
        Value::Bool(x) => format!("bool(@{:04x}:{})", short_addr(x.addr()), *x.borrow()),
        #[cfg(feature = "string")]
        Value::String(x) => format!("str(@{:04x}:\"{}\")", short_addr(x.addr()), x.borrow()),
        #[cfg(feature = "atom")]
        Value::Atom(x) => format!("{}(@{:04x})", x.borrow().to_string(), short_addr(x.addr())),
        #[cfg(feature = "tuple")]
        Value::Tuple(tuple_ref) => summarize_tuple_value(tuple_ref, depth),
        _ => format!(
            "{}({})",
            value.kind().to_string(),
            truncate_for_trace(&single_line_trace_text(&format!("{:?}", value)), 48)
        ),
    }
}

#[cfg(feature = "tuple")]
fn summarize_tuple_value(tuple_ref: &Ref<MechTuple>, depth: usize) -> String {
    let tuple = tuple_ref.borrow();
    let mut parts = Vec::new();
    for element in tuple.elements.iter().take(3) {
        parts.push(summarize_value_compact(element, depth + 1));
    }
    if tuple.elements.len() > 3 {
        parts.push("…".to_string());
    }
    format!(
        "tuple(@{:04x}; len={}; [{}])",
        short_addr(tuple_ref.addr()),
        tuple.elements.len(),
        parts.join(", ")
    )
}

fn short_addr(addr: usize) -> u16 {
    (addr & 0xffff) as u16
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
        Pattern::Array(array) => {
            let spread = if array.spread.is_some() { ",spread" } else { "" };
            format!(
                "array(prefix={}{} ,suffix={})",
                array.prefix.len(),
                spread,
                array.suffix.len()
            )
        }
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
        Pattern::Array(pattern_array) => {
            let detached = detach_value(value);
            let values = match matrix_like_values(&detached) {
                Some(values) => values,
                None => return Ok(false),
            };
            if values.len() < pattern_array.prefix.len() + pattern_array.suffix.len() {
                return Ok(false);
            }

            for (pat, val) in pattern_array.prefix.iter().zip(values.iter()) {
                if !pattern_matches_value(pat, val, env, p)? {
                    return Ok(false);
                }
            }

            let suffix_start = values.len() - pattern_array.suffix.len();
            for (pat, val) in pattern_array
                .suffix
                .iter()
                .zip(values[suffix_start..].iter())
            {
                if !pattern_matches_value(pat, val, env, p)? {
                    return Ok(false);
                }
            }

            if pattern_array.spread.is_none() && values.len() != pattern_array.prefix.len() + pattern_array.suffix.len() {
                return Ok(false);
            }

            if let Some(spread) = &pattern_array.spread {
                if let Some(binding) = &spread.binding {
                    let middle = values[pattern_array.prefix.len()..suffix_start].to_vec();
                    let captured = Value::MatrixValue(Matrix::from_vec(middle, 1, suffix_start.saturating_sub(pattern_array.prefix.len())));
                    if !pattern_matches_value(binding, &captured, env, p)? {
                        return Ok(false);
                    }
                }
            }

            Ok(true)
        }
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

fn matrix_like_values(value: &Value) -> Option<Vec<Value>> {
    match value {
        #[cfg(feature = "matrix")]
        Value::MatrixIndex(matrix) => Some(matrix.as_vec().into_iter().map(|value| Value::Index(Ref::new(value))).collect()),
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
        Value::MatrixR64(matrix) => Some(matrix.as_vec().into_iter().map(|value| value.to_value()).collect()),
        #[cfg(all(feature = "matrix", feature = "complex"))]
        Value::MatrixC64(matrix) => Some(matrix.as_vec().into_iter().map(|value| value.to_value()).collect()),
        #[cfg(feature = "matrix")]
        Value::MatrixValue(matrix) => Some(matrix.as_vec()),
        _ => None,
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
