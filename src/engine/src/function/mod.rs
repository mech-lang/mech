use crate::tracing::{
    format_trace, format_trace_args, summarize_function_pattern, summarize_function_value,
    summarize_values_with_kinds,
};
use crate::*;
#[cfg(all(feature = "kind_annotation", feature = "enum"))]
use std::collections::HashSet;
use std::sync::Arc;

#[cfg(feature = "source")]
pub use crate::expressions::function_call;

// Functions
// ============================================================================

// Frames
// ----------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FrameState {
    Running,
    Suspended,
    Completed,
}

// One activation record on the call stack. Every user-function invocation gets
// its own Frame so locals and the instruction pointer don't bleed across calls.
#[derive(Clone)]
pub struct Frame {
    plan: Plan,
    ip: usize,                // index of the next instruction to execute
    locals: SymbolTableRef,   // variables local to this invocation
    out: Option<LegacyValue>, // value yielded by a coroutine, if any
    state: FrameState,        // Running / Suspended / Completed
}

impl Frame {
    pub(crate) fn checkpoint_plan(&self) -> Plan {
        self.plan.clone()
    }

    pub(crate) fn checkpoint_locals(&self) -> SymbolTableRef {
        self.locals.clone()
    }

    pub(crate) fn checkpoint_out(&self) -> Option<LegacyValue> {
        self.out.clone()
    }
}

// The call stack is a simple growable list of frames; the last entry is current.
#[derive(Clone)]
pub struct Stack {
    frames: Vec<Frame>,
}

// Registers a user-written function so it can be called by name later.
// Hashes the name to a u64 id used as the lookup key throughout the runtime.
#[cfg(feature = "source")]
mod source_only {
    use super::*;

    pub fn function_define(
        fxn_def: &FunctionDefine,
        p: &InterpreterExecution<'_>,
    ) -> MResult<FunctionDefinition> {
        let fxn_name_id = fxn_def.name.hash();
        let mut new_fxn =
            FunctionDefinition::new(fxn_name_id, fxn_def.name.to_string(), fxn_def.clone());

        // Record declared input arguments and their kind annotations.
        for input_arg in &fxn_def.input {
            new_fxn
                .input
                .insert(input_arg.name.hash(), input_arg.kind.clone());
        }

        // Record declared output arguments and their kind annotations.
        for output_arg in &fxn_def.output {
            new_fxn
                .output
                .insert(output_arg.name.hash(), output_arg.kind.clone());
        }

        // User definitions are checkpointed program state, separate from the
        // immutable runtime catalog.
        let mut state = p.state.borrow_mut();
        state.user_functions.insert_or_replace(new_fxn.clone())?;
        state
            .dictionary
            .borrow_mut()
            .insert(fxn_name_id, fxn_def.name.to_string());

        Ok(new_fxn)
    }

    // Calls
    // ----------------------------------------------------------------------------

    // Asks a function specializer to select the right concrete implementation
    // for the given argument types, runs it once to produce an initial value, then
    // pushes it onto the reactive plan so it re-runs when its inputs change.
    pub fn execute_function_specializer(
        specializer: Arc<dyn FunctionSpecializer>,
        input_arg_values: &Vec<LegacyValue>,
        p: &InterpreterExecution<'_>,
    ) -> MResult<LegacyValue> {
        let new_fxn = specializer.specialize(input_arg_values)?;
        execute_specialized_function(new_fxn, input_arg_values, p)
    }

    /// Runs and registers a function selected by either the explicit catalog or a
    /// program-local function extension.
    pub fn execute_specialized_function(
        new_fxn: Box<dyn MechFunction>,
        input_arg_values: &Vec<LegacyValue>,
        p: &InterpreterExecution<'_>,
    ) -> MResult<LegacyValue> {
        let plan = p.plan();
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
        solve_specialized_initial_output(new_fxn.as_ref(), &plan, p)?;
        let result = new_fxn.out();
        trace_println!(
            p,
            "{}",
            format_trace(
                "arm",
                format!("result {}", summarize_function_value(&result))
            )
        );
        plan.register_function(new_fxn, input_arg_values)?;
        Ok(result)
    }

    pub fn execute_initialized_indexed_compiler_with_registration_arguments(
        p: &InterpreterExecution<'_>,
        plan: &Plan,
        compiler: &dyn FunctionSpecializer,
        compile_arguments: Vec<LegacyValue>,
        registration_arguments: Vec<LegacyValue>,
    ) -> MResult<LegacyValue> {
        let function = compiler.specialize(&compile_arguments)?;
        solve_specialized_initial_output(function.as_ref(), plan, p)?;
        let output = function.out();
        plan.register_function(function, &registration_arguments)?;
        Ok(output)
    }

    pub(crate) fn execute_initialized_indexed_compiler(
        p: &InterpreterExecution<'_>,
        plan: &Plan,
        compiler: &dyn FunctionSpecializer,
        arguments: Vec<LegacyValue>,
    ) -> MResult<LegacyValue> {
        let registration_arguments = arguments.clone();
        execute_initialized_indexed_compiler_with_registration_arguments(
            p,
            plan,
            compiler,
            arguments,
            registration_arguments,
        )
    }

    pub fn execute_catalog_operation_with_registration_arguments(
        p: &InterpreterExecution<'_>,
        plan: &Plan,
        canonical_name: &str,
        compile_arguments: Vec<LegacyValue>,
        registration_arguments: Vec<LegacyValue>,
    ) -> MResult<LegacyValue> {
        let operation = OperationId::from_name(canonical_name);
        let function = p.specialize_visible_operation_named(
            operation,
            Some(canonical_name),
            &compile_arguments,
        )?;
        solve_specialized_initial_output(function.as_ref(), plan, p)?;
        let output = function.out();
        plan.register_function(function, &registration_arguments)?;
        Ok(output)
    }

    pub(crate) fn execute_catalog_operation(
        p: &InterpreterExecution<'_>,
        plan: &Plan,
        canonical_name: &str,
        arguments: Vec<LegacyValue>,
    ) -> MResult<LegacyValue> {
        let registration_arguments = arguments.clone();
        execute_catalog_operation_with_registration_arguments(
            p,
            plan,
            canonical_name,
            arguments,
            registration_arguments,
        )
    }

    fn solve_specialized_initial_output(
        function: &dyn MechFunction,
        plan: &Plan,
        p: &InterpreterExecution<'_>,
    ) -> MResult<()> {
        if !plan.activation_registration_active() {
            match function.initial_solve_policy() {
                InitialSolvePolicy::Solve => {
                    p.with_services(|services| function.solve_result_with(services))?;
                }
                InitialSolvePolicy::PreserveSpecializedOutput => {
                    p.with_services(|services| {
                        function.initialize_preserved_output_with(services)
                    })?;
                }
            }
        }
        Ok(())
    }

    // Executes a user-defined function. Handles argument count validation,
    // optional matrix broadcasting, match-arm dispatch, and plain statement bodies.
    // Logs entry/exit (or failure) via the trace machinery.
    pub(crate) fn execute_user_function(
        fxn_def: &FunctionDefinition,
        input_arg_values: &Vec<LegacyValue>,
        p: &InterpreterExecution<'_>,
    ) -> MResult<LegacyValue> {
        // Reject calls with the wrong number of arguments before doing anything else.
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

        // If the function takes a single matrix argument and the element kind matches
        // the output kind, broadcast element-wise instead of running the body once.
        #[cfg(feature = "matrix")]
        if let Some(result) = try_broadcast_user_function(fxn_def, input_arg_values, p)? {
            return Ok(result);
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

        // Choose execution strategy: match-arm body vs. plain statement body.
        let output = if !fxn_def.code.match_arms.is_empty() {
            // Match-arm body: loop to support tail-call optimisation. Each iteration
            // opens a fresh scope, binds the current arguments, runs the arms, then
            // either returns the result or loops with a new argument set.
            let mut current_args: Vec<LegacyValue> = input_arg_values.clone();
            loop {
                let scope = FunctionScope::enter(p);
                bind_function_inputs(fxn_def, &current_args, p)?;
                let step: FunctionCallStep =
                    execute_function_match_arms(fxn_def, &current_args, p)?;
                drop(scope);
                match step {
                    FunctionCallStep::Return(value) => break Ok(value),
                    // Tail call: swap in the new args and go around again without growing
                    // the Rust call stack.
                    FunctionCallStep::TailCall(next_args) => {
                        current_args = next_args;
                    }
                }
            }
        } else {
            // Plain statement body: run statements in order, then collect named outputs.
            let scope = FunctionScope::enter(p);
            bind_function_inputs(fxn_def, input_arg_values, p)?;
            for statement_node in &fxn_def.code.statements {
                statement(statement_node, None, p)?;
            }
            let result = collect_function_output(p, fxn_def);
            drop(scope);
            result
        };

        match output {
            Ok(value) => {
                trace_println!(
                    p,
                    "{}",
                    format_trace(
                        "fn",
                        format!(
                            "exit  {} => {}",
                            fxn_def.name,
                            summarize_function_value(&value)
                        )
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

    // The outcome of executing one match arm. Either we have a final value, or
    // we identified a tail call and carry its new arguments for the next iteration.
    enum FunctionCallStep {
        Return(LegacyValue),
        TailCall(Vec<LegacyValue>),
    }

    // Lift a function over one or more matrix-backed outer collections. An
    // argument that already conforms to its declared kind is shared by every
    // lane; otherwise, a matrix whose elements conform supplies one value per
    // lane. Every lifted argument must use the same outer shape.
    #[cfg(feature = "matrix")]
    fn try_broadcast_user_function(
        fxn_def: &FunctionDefinition,
        input_arg_values: &Vec<LegacyValue>,
        p: &InterpreterExecution<'_>,
    ) -> MResult<Option<LegacyValue>> {
        // Resolve the declared input and output kinds from their annotations.
        // Without kind_annotation feature we can't know the element type, so bail.
        #[cfg(feature = "kind_annotation")]
        let (input_kinds, output_kinds) = {
            let kinds = &p.state.borrow().kinds;
            let input_kinds = fxn_def
                .code
                .input
                .iter()
                .map(|input| kind_annotation(&input.kind.kind, p)?.to_value_kind(kinds))
                .collect::<MResult<Vec<_>>>()?;
            let output_kinds = fxn_def
                .code
                .output
                .iter()
                .map(|output| kind_annotation(&output.kind.kind, p)?.to_value_kind(kinds))
                .collect::<MResult<Vec<_>>>()?;
            (input_kinds, output_kinds)
        };

        #[cfg(not(feature = "kind_annotation"))]
        let (input_kinds, output_kinds) = {
            return Ok(None);
        };

        if input_kinds.len() != input_arg_values.len() || output_kinds.is_empty() {
            return Ok(None);
        }

        enum LiftedArgument {
            Shared(LegacyValue),
            PerLane(Vec<LegacyValue>),
        }

        let mut outer_shape: Option<(usize, usize)> = None;
        let mut arguments = Vec::with_capacity(input_arg_values.len());
        for (argument, declared_kind) in input_arg_values.iter().zip(&input_kinds) {
            let argument = detach_value(argument);
            if value_conforms_to_kind(&argument, declared_kind) {
                arguments.push(LiftedArgument::Shared(argument));
                continue;
            }
            let Some(elements) = crate::patterns::matrix_like_values(&argument) else {
                return Ok(None);
            };
            if !elements
                .iter()
                .all(|element| value_conforms_to_kind(element, declared_kind))
            {
                return Ok(None);
            }
            let shape = argument.shape();
            let shape = (shape[0], shape[1]);
            match outer_shape {
                Some(expected) if expected != shape => {
                    return Err(MechError::new(
                        GenericError {
                            msg: format!(
                                "function `{}` cannot lift arguments with outer shapes {}x{} and {}x{}",
                                fxn_def.name, expected.0, expected.1, shape.0, shape.1,
                            ),
                        },
                        None,
                    )
                    .with_compiler_loc()
                    .with_tokens(fxn_def.code.name.tokens()));
                }
                None => outer_shape = Some(shape),
                _ => {}
            }
            arguments.push(LiftedArgument::PerLane(elements));
        }

        let Some((rows, columns)) = outer_shape else {
            return Ok(None);
        };
        let lane_count = rows.checked_mul(columns).ok_or_else(|| {
            MechError::new(
                GenericError {
                    msg: format!("function `{}` outer shape overflowed", fxn_def.name),
                },
                None,
            )
            .with_compiler_loc()
            .with_tokens(fxn_def.code.name.tokens())
        })?;
        let mut outputs = (0..output_kinds.len())
            .map(|_| Vec::with_capacity(lane_count))
            .collect::<Vec<_>>();

        for lane in 0..lane_count {
            let lane_arguments = arguments
                .iter()
                .map(|argument| match argument {
                    LiftedArgument::Shared(value) => value.clone(),
                    LiftedArgument::PerLane(values) => values[lane].clone(),
                })
                .collect::<Vec<_>>();
            let lane_output = execute_user_function(fxn_def, &lane_arguments, p)?;
            if outputs.len() == 1 {
                outputs[0].push(lane_output);
                continue;
            }
            let LegacyValue::Tuple(tuple) = lane_output else {
                return Err(MechError::new(
                    GenericError {
                        msg: format!(
                            "function `{}` returned a non-tuple while lifting {} outputs",
                            fxn_def.name,
                            outputs.len(),
                        ),
                    },
                    None,
                )
                .with_compiler_loc()
                .with_tokens(fxn_def.code.name.tokens()));
            };
            let tuple = tuple.borrow();
            if tuple.elements.len() != outputs.len() {
                return Err(MechError::new(
                    GenericError {
                        msg: format!(
                            "function `{}` returned {} values while lifting {} outputs",
                            fxn_def.name,
                            tuple.elements.len(),
                            outputs.len(),
                        ),
                    },
                    None,
                )
                .with_compiler_loc()
                .with_tokens(fxn_def.code.name.tokens()));
            }
            for (output, value) in outputs.iter_mut().zip(&tuple.elements) {
                output.push(value.as_ref().clone());
            }
        }

        let mut lifted_outputs = outputs
            .into_iter()
            .zip(output_kinds)
            .map(|(values, kind)| build_typed_matrix_from_values(&kind, values, rows, columns))
            .collect::<Vec<_>>();
        Ok(Some(if lifted_outputs.len() == 1 {
            lifted_outputs.remove(0)
        } else {
            LegacyValue::Tuple(Ref::new(MechTuple::from_vec(lifted_outputs)))
        }))
    }

    #[cfg(feature = "matrix")]
    fn value_conforms_to_kind(value: &LegacyValue, kind: &ValueKind) -> bool {
        matches!(kind, ValueKind::Any)
            || value.kind() == *kind
            || value.clone().convert_to(kind).is_some()
    }

    // Assembles a list of scalar Values into a typed matrix.
    // TODO add more types
    #[cfg(feature = "matrix")]
    fn build_typed_matrix_from_values(
        output_kind: &ValueKind,
        outputs: Vec<LegacyValue>,
        rows: usize,
        cols: usize,
    ) -> LegacyValue {
        match output_kind {
            #[cfg(feature = "f64")]
            ValueKind::F64 => LegacyValue::MatrixF64(f64::to_matrix(
                outputs
                    .into_iter()
                    .map(|value| {
                        value
                            .as_f64()
                            .expect("Expected f64 output")
                            .borrow()
                            .clone()
                    })
                    .collect::<Vec<f64>>(),
                rows,
                cols,
            )),
            _ => LegacyValue::MatrixValue(LegacyValue::to_matrix(outputs, rows, cols)),
        }
    }

    // Tries each match arm in order against the current arguments. Handles:
    //   - enum exhaustiveness checking (kind_annotation + enum features)
    //   - tail-call detection (arm body is a recursive call with same arity)
    //   - output kind coercion
    // Returns an error if no arm matched.
    fn execute_function_match_arms(
        fxn_def: &FunctionDefinition,
        input_arg_values: &Vec<LegacyValue>,
        p: &InterpreterExecution<'_>,
    ) -> MResult<FunctionCallStep> {
        // Exhaustiveness check: when the single input is an enum type and there is
        // no wildcard arm, every variant must be covered or we report which ones
        // are missing before even attempting to run.
        #[cfg(all(feature = "kind_annotation", feature = "enum"))]
        {
            let has_wildcard = fxn_def
                .code
                .match_arms
                .iter()
                .any(|arm| matches!(arm.pattern, Pattern::Wildcard));
            if !has_wildcard && fxn_def.input.len() == 1 {
                if let Some((_, kind_annotation_node)) = fxn_def.input.iter().next() {
                    let input_kind = kind_annotation(&kind_annotation_node.kind, p)?
                        .to_value_kind(&p.state.borrow().kinds)?;
                    if let ValueKind::Enum(enum_id, _) = input_kind {
                        let state_brrw = p.state.borrow();
                        if let Some(enum_def) = state_brrw.enums.get(&enum_id) {
                            // Collect every variant name that appears in the written arms.
                            let mut covered_variants: HashSet<u64> = HashSet::new();
                            for arm in &fxn_def.code.match_arms {
                                match &arm.pattern {
                                    #[cfg(feature = "atom")]
                                    Pattern::TupleStruct(tuple_struct) => {
                                        covered_variants.insert(tuple_struct.name.hash());
                                    }
                                    Pattern::Expression(expr) => {
                                        if let Expression::Literal(Literal::Atom(atom)) = expr {
                                            covered_variants.insert(atom.name.hash());
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            let all_covered = enum_def
                                .variants
                                .iter()
                                .all(|(variant_id, _)| covered_variants.contains(variant_id));
                            if !all_covered {
                                // Build a readable list of the missing variant patterns.
                                let missing_patterns = enum_def
                                    .variants
                                    .iter()
                                    .filter(|(variant_id, _)| {
                                        !covered_variants.contains(variant_id)
                                    })
                                    .map(|(variant_id, payload_kind)| {
                                        let variant_name = enum_def
                                            .names
                                            .borrow()
                                            .get(variant_id)
                                            .cloned()
                                            .unwrap_or_else(|| variant_id.to_string());
                                        if payload_kind.is_some() {
                                            format!(":{}(…)", variant_name)
                                        } else {
                                            format!(":{}", variant_name)
                                        }
                                    })
                                    .collect::<Vec<String>>();
                                return Err(MechError::new(
                                    FunctionMatchNonExhaustiveError {
                                        function_name: fxn_def.name.clone(),
                                        missing_patterns,
                                    },
                                    None,
                                )
                                .with_compiler_loc()
                                .with_tokens(fxn_def.code.name.tokens()));
                            }
                        }
                    }
                }
            }
        }

        // Try each arm in source order; the first one whose pattern matches wins.
        for (arm_idx, arm) in fxn_def.code.match_arms.iter().enumerate() {
            let mut env = Environment::new();
            let matched = crate::patterns::pattern_matches_arguments(
                &arm.pattern,
                input_arg_values,
                &mut env,
                p,
            )?;
            trace_println!(p, "{}", {
                let args_summary = summarize_values_with_kinds(input_arg_values);
                let pattern_summary = summarize_function_pattern(&arm.pattern);
                let marker = if matched { "✓" } else { "X" };
                format_trace(
                    "match",
                    format!(
                        "arm[{arm_idx}] test pattern={pattern_summary} args=[{args_summary}] {marker}"
                    ),
                )
            });
            if matched {
                // Tail-call optimisation: if the arm body is a direct recursive call
                // with the same arity, return new arguments instead of recursing.
                if let Expression::FunctionCall(fxn_call) = &arm.expression {
                    if fxn_call.name.hash() == fxn_def.code.name.hash() {
                        let mut tail_args = Vec::with_capacity(fxn_call.args.len());
                        for (_, arg_expr) in fxn_call.args.iter() {
                            tail_args.push(expression(arg_expr, Some(&env), p)?);
                        }
                        if tail_args.len() == fxn_def.input.len() {
                            trace_println!(
                                p,
                                "{}",
                                format_trace(
                                    "match",
                                    format!("arm[{arm_idx}] tail-call {}", fxn_def.name)
                                )
                            );
                            return Ok(FunctionCallStep::TailCall(tail_args));
                        }
                    }
                }
                // Normal arm: evaluate the expression and coerce to the declared output kind.
                let coerced = detach_value(&expression(&arm.expression, Some(&env), p)?);
                #[cfg(feature = "kind_annotation")]
                let coerced = coerce_function_output_kind(coerced, fxn_def, p)?;
                trace_println!(
                    p,
                    "{}",
                    format_trace(
                        "match",
                        format!(
                            "arm[{arm_idx}] out  value={} kind={}",
                            summarize_function_value(&coerced),
                            coerced.kind().to_string()
                        )
                    )
                );
                return Ok(FunctionCallStep::Return(coerced));
            }
        }
        // No arm matched — this is a runtime error; the function has no defined output.
        Err(MechError::new(
            FunctionOutputUndefinedError {
                output_id: fxn_def.id,
            },
            None,
        )
        .with_compiler_loc()
        .with_tokens(fxn_def.code.name.tokens()))
    }

    // Coerces a match-arm result to the function's declared output kind.
    // If no output annotation exists, or conversion fails, the value is returned as-is.
    #[cfg(feature = "kind_annotation")]
    fn coerce_function_output_kind(
        value: LegacyValue,
        fxn_def: &FunctionDefinition,
        p: &InterpreterExecution<'_>,
    ) -> MResult<LegacyValue> {
        if fxn_def.output.is_empty() {
            return Ok(value);
        }
        let Some((_, output_kind_annotation)) = fxn_def.output.get_index(0) else {
            return Ok(value);
        };
        let target_kind = kind_annotation(&output_kind_annotation.kind, p)?
            .to_value_kind(&p.state.borrow().kinds)?;
        return Ok(value.convert_to(&target_kind).unwrap_or(value));
    }

    // RAII guard that swaps in a fresh symbol table and plan for the duration of a
    // function call, then restores the previous ones on drop. This is what gives
    // each function its own local variable namespace.
    struct FunctionScope {
        state: Ref<ProgramState>,
        previous_symbols: SymbolTableRef,
        previous_plan: Option<Plan>,
        previous_environment: Option<SymbolTableRef>,
    }

    impl FunctionScope {
        fn enter(p: &InterpreterExecution<'_>) -> Self {
            let state = p.state.clone();
            let mut state_brrw = state.borrow_mut();
            // A new symbol table that shares the global name dictionary so that
            // lookups by hash still resolve to human-readable names.
            let mut local_symbols = SymbolTable::new();
            local_symbols.dictionary = state_brrw.dictionary.clone();
            let local_symbols = Ref::new(local_symbols);
            let previous_symbols = std::mem::replace(&mut state_brrw.symbol_table, local_symbols);
            let previous_plan = if *p.persistent_user_function_plan_depth.borrow() > 0 {
                None
            } else {
                Some(std::mem::replace(&mut state_brrw.plan, Plan::new()))
            };
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

    // Restore the caller's symbol table, plan, and environment when the scope ends.
    impl Drop for FunctionScope {
        fn drop(&mut self) {
            let mut state_brrw = self.state.borrow_mut();
            state_brrw.symbol_table = self.previous_symbols.clone();
            if let Some(previous_plan) = &self.previous_plan {
                state_brrw.plan = previous_plan.clone();
            }
            state_brrw.environment = self.previous_environment.clone();
        }
    }

    pub(crate) struct PersistentUserFunctionPlanScope {
        depth: Ref<usize>,
    }

    impl PersistentUserFunctionPlanScope {
        pub(crate) fn enter(interpreter: &Interpreter) -> Self {
            let depth = interpreter.persistent_user_function_plan_depth.clone();
            *depth.borrow_mut() += 1;
            Self { depth }
        }
    }

    impl Drop for PersistentUserFunctionPlanScope {
        fn drop(&mut self) {
            let mut depth = self.depth.borrow_mut();
            debug_assert!(*depth > 0);
            *depth -= 1;
        }
    }

    // Function Definitions
    // ----------------------------------------------------------------------------

    // Binds each argument value to the corresponding local variable name.
    // With kind_annotation: validates and coerces argument types, including
    // special handling for enum types where coercion rules differ.
    fn bind_function_inputs(
        fxn_def: &FunctionDefinition,
        input_arg_values: &Vec<LegacyValue>,
        p: &InterpreterExecution<'_>,
    ) -> MResult<()> {
        let scoped_state = p.state.borrow();
        for ((arg_id, input_kind_annotation), input_value) in
            fxn_def.input.iter().zip(input_arg_values.iter())
        {
            // Look up the human-readable argument name for error messages.
            let arg_name = fxn_def
                .code
                .input
                .iter()
                .find(|arg| arg.name.hash() == *arg_id)
                .map(|arg| arg.name.to_string())
                .unwrap_or_else(|| arg_id.to_string());

            let bound_value = {
                #[cfg(feature = "kind_annotation")]
                {
                    let target_kind = kind_annotation(&input_kind_annotation.kind, p)?
                        .to_value_kind(&p.state.borrow().kinds)?;
                    let detached_input = detach_value(input_value);

                    // Enum arguments are checked for membership rather than converted,
                    // because coercion semantics don't apply across enum variants.
                    #[cfg(all(feature = "enum", feature = "atom"))]
                    if let ValueKind::Enum(enum_id, _) = &target_kind {
                        let state_brrw = p.state.borrow();
                        if enum_value_matches(detached_input.clone(), *enum_id, &state_brrw) {
                            detached_input.clone()
                        } else {
                            return Err(MechError::new(
                                FunctionInputTypeMismatchError {
                                    function_name: fxn_def.name.clone(),
                                    argument_name: arg_name.clone(),
                                    expected: target_kind.clone(),
                                    found: detached_input.kind(),
                                },
                                None,
                            )
                            .with_compiler_loc()
                            .with_tokens(input_kind_annotation.tokens()));
                        }
                    } else {
                        // Non-enum: attempt type conversion; error if it can't be done.
                        detached_input
                            .clone()
                            .convert_to(&target_kind)
                            .ok_or_else(|| {
                                MechError::new(
                                    FunctionInputTypeMismatchError {
                                        function_name: fxn_def.name.clone(),
                                        argument_name: arg_name.clone(),
                                        expected: target_kind.clone(),
                                        found: detached_input.kind(),
                                    },
                                    None,
                                )
                                .with_compiler_loc()
                                .with_tokens(input_kind_annotation.tokens())
                            })?
                    }
                    #[cfg(not(all(feature = "enum", feature = "atom")))]
                    detached_input
                        .clone()
                        .convert_to(&target_kind)
                        .ok_or_else(|| {
                            MechError::new(
                                FunctionInputTypeMismatchError {
                                    function_name: fxn_def.name.clone(),
                                    argument_name: arg_name.clone(),
                                    expected: target_kind.clone(),
                                    found: detached_input.kind(),
                                },
                                None,
                            )
                            .with_compiler_loc()
                            .with_tokens(input_kind_annotation.tokens())
                        })?
                }
                // Without kind_annotation: accept the value as-is, just detach any reference.
                #[cfg(not(feature = "kind_annotation"))]
                {
                    detach_value(input_value)
                }
            };
            #[cfg(feature = "subscript_formula")]
            if current_string_access_expression_live(p)
                || string_access_input_is_live(input_value, p)
            {
                mark_string_access_value_live(p, &bound_value);
            }
            scoped_state.save_symbol(*arg_id, arg_name, bound_value, false);
        }
        Ok(())
    }

    // Returns true if `value` is a valid member of the enum identified by `enum_id`.
    // Handles bare atom variants and tuple-struct variants (atom tag + payload).
    #[cfg(all(feature = "enum", feature = "atom"))]
    fn enum_value_matches(value: LegacyValue, enum_id: u64, state: &ProgramState) -> bool {
        let enum_def = match state.enums.get(&enum_id) {
            Some(enm) => enm,
            None => return false,
        };
        let names_brrw = enum_def.names.borrow();
        let atom_matches_variant = |variant_id: u64, atom_id: u64, atom_name: &str| {
            if variant_id == atom_id {
                return true;
            }
            let variant_name = match names_brrw.get(&variant_id) {
                Some(name) => name.as_str(),
                None => return false,
            };
            let short_variant = variant_name.rsplit('/').next().unwrap_or(variant_name);
            let short_atom = atom_name.rsplit('/').next().unwrap_or(atom_name);
            short_variant == short_atom
        };
        match value {
            LegacyValue::Enum(enum_value) => {
                let enum_value_brrw = enum_value.borrow();
                if enum_value_brrw.id != enum_id {
                    return false;
                }
                if enum_value_brrw.variants.len() != 1 {
                    return false;
                }
                let (variant_id, payload) = &enum_value_brrw.variants[0];
                let (_, declared_payload_kind) = match enum_def
                    .variants
                    .iter()
                    .find(|(known_variant, _)| *known_variant == *variant_id)
                {
                    Some(entry) => entry,
                    None => return false,
                };
                match (payload, declared_payload_kind) {
                    (None, None) => true,
                    (Some(payload_value), Some(LegacyValue::Kind(expected_kind))) => {
                        match expected_kind {
                            ValueKind::Enum(inner_enum_id, _) => {
                                enum_value_matches(payload_value.clone(), *inner_enum_id, state)
                            }
                            _ => {
                                payload_value.kind() == expected_kind.clone()
                                    || payload_value.convert_to(expected_kind).is_some()
                            }
                        }
                    }
                    _ => false,
                }
            }
            // Bare atom: check that the atom's id is a known payload-less variant.
            LegacyValue::Atom(atom) => {
                let atom_brrw = atom.borrow();
                let variant_id = atom_brrw.id();
                let atom_name = atom_brrw.name();
                enum_def
                    .variants
                    .iter()
                    .any(|(known_variant, payload_kind)| {
                        atom_matches_variant(*known_variant, variant_id, &atom_name)
                            && payload_kind.is_none()
                    })
            }
            // Tuple-struct variant: a 2-element tuple of (atom-tag, payload).
            // The tag must match a known variant and the payload must satisfy the
            // declared payload kind, recursing for nested enums.
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(tuple_val) => {
                let tuple_brrw = tuple_val.borrow();
                if tuple_brrw.elements.len() != 2 {
                    return false;
                }
                let (tag, tag_name) = match tuple_brrw.elements[0].as_ref() {
                    LegacyValue::Atom(atom) => {
                        let atom_brrw = atom.borrow();
                        (atom_brrw.id(), atom_brrw.name())
                    }
                    _ => return false,
                };
                let payload = tuple_brrw.elements[1].as_ref().clone();
                let (_, declared_payload_kind) =
                    match enum_def.variants.iter().find(|(known_variant, _)| {
                        atom_matches_variant(*known_variant, tag, &tag_name)
                    }) {
                        Some(entry) => entry,
                        None => return false,
                    };
                match declared_payload_kind {
                    Some(LegacyValue::Kind(expected_kind)) => match expected_kind {
                        // Nested enum payload: recurse.
                        ValueKind::Enum(inner_enum_id, _) => {
                            enum_value_matches(payload, *inner_enum_id, state)
                        }
                        // Scalar payload: accept exact match or a convertible value.
                        _ => {
                            payload.kind() == expected_kind.clone()
                                || payload.convert_to(expected_kind).is_some()
                        }
                    },
                    _ => false,
                }
            }
            _ => false,
        }
    }

    // Reads each declared output variable out of the local symbol table and
    // returns them as a single Value. Multiple outputs are wrapped in a Tuple;
    // a single output is returned directly; zero outputs return Empty.
    fn collect_function_output(
        p: &InterpreterExecution<'_>,
        fxn_def: &FunctionDefinition,
    ) -> MResult<LegacyValue> {
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
            0 => LegacyValue::Empty,
            1 => outputs.remove(0),
            #[cfg(feature = "tuple")]
            _ => LegacyValue::Tuple(Ref::new(MechTuple::from_vec(outputs))),
            #[cfg(not(feature = "tuple"))]
            _ => {
                return Err(MechError::new(FeatureNotEnabledError, None)
                    .with_compiler_loc()
                    .with_tokens(fxn_def.code.name.tokens()));
            }
        })
    }

    // Peels off any MutableReference wrappers to get to the underlying value.
    // Used before storing arguments or returning results so callers always see
    // plain owned values, not live references into other cells.
    pub(crate) fn detach_value(value: &LegacyValue) -> LegacyValue {
        match value {
            LegacyValue::MutableReference(reference) => detach_value(&reference.borrow()),
            _ => value.clone(),
        }
    }

    // Function Errors
    // ----------------------------------------------------------------------------

    // A function's output variable was declared but never assigned during execution.
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

    // A match-arm function doesn't cover every variant of its enum input type.
    #[derive(Debug, Clone)]
    pub struct FunctionMatchNonExhaustiveError {
        pub function_name: String,
        pub missing_patterns: Vec<String>,
    }

    impl MechErrorKind for FunctionMatchNonExhaustiveError {
        fn name(&self) -> &str {
            "FunctionMatchNonExhaustive"
        }

        fn message(&self) -> String {
            format!(
                "Function '{}' has non-exhaustive match arms. Missing patterns: {}. Add the missing patterns or add a wildcard (`*`) arm.",
                self.function_name,
                self.missing_patterns.join(", ")
            )
        }
    }

    // A value passed to a function argument didn't match the declared kind and
    // couldn't be coerced to it.
    #[derive(Debug, Clone)]
    pub struct FunctionInputTypeMismatchError {
        pub function_name: String,
        pub argument_name: String,
        pub expected: ValueKind,
        pub found: ValueKind,
    }

    impl MechErrorKind for FunctionInputTypeMismatchError {
        fn name(&self) -> &str {
            "FunctionInputTypeMismatch"
        }

        fn message(&self) -> String {
            format!(
                "Function '{}' argument '{}' expected {}, found {}",
                self.function_name, self.argument_name, self.expected, self.found
            )
        }
    }

    #[cfg(all(test, feature = "functions", feature = "f64"))]
    mod native_dependency_tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct NativeDependencyTestCompiler;

        impl FunctionSpecializer for NativeDependencyTestCompiler {
            fn specialize(&self, _arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
                Ok(Box::new(NativeDependencyTestFunction {
                    output: LegacyValue::F64(Ref::new(2.0)),
                }))
            }
        }

        struct NativeDependencyTestFunction {
            output: LegacyValue,
        }

        impl MechFunctionImpl for NativeDependencyTestFunction {
            fn solve_result(&self) -> MResult<()> {
                Ok(())
            }

            fn out(&self) -> LegacyValue {
                self.output.clone()
            }

            fn to_string(&self) -> String {
                "native-dependency-test".to_string()
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }

        #[cfg(feature = "compiler")]
        impl MechFunctionCompiler for NativeDependencyTestFunction {
            fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                Ok(0)
            }
        }

        struct IndexedInitializedCompiler {
            output: f64,
            solve_calls: Arc<AtomicUsize>,
        }

        struct IndexedInitializedFunction {
            output: LegacyValue,
            solve_calls: Arc<AtomicUsize>,
        }

        impl FunctionSpecializer for IndexedInitializedCompiler {
            fn specialize(&self, _arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
                Ok(Box::new(IndexedInitializedFunction {
                    output: LegacyValue::F64(Ref::new(self.output)),
                    solve_calls: self.solve_calls.clone(),
                }))
            }
        }

        impl MechFunctionImpl for IndexedInitializedFunction {
            fn solve_result(&self) -> MResult<()> {
                self.solve_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }

            fn out(&self) -> LegacyValue {
                self.output.clone()
            }

            fn to_string(&self) -> String {
                "indexed-initialized-test".to_string()
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }

        #[cfg(feature = "compiler")]
        impl MechFunctionCompiler for IndexedInitializedFunction {
            fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                Ok(0)
            }
        }

        #[test]
        fn initialized_indexed_compiler_records_dependencies() {
            let interpreter = Interpreter::new(0, 100);
            let plan = interpreter.plan();
            let mut services = NoMechExecutionServices;
            let execution = InterpreterExecution::new(&interpreter, &mut services);
            let input = Ref::new(1.0);
            let input_cell = ReactiveCellId::new(input.id());
            let solve_calls = Arc::new(AtomicUsize::new(0));

            let result = execute_initialized_indexed_compiler(
                &execution,
                &plan,
                &IndexedInitializedCompiler {
                    output: 2.0,
                    solve_calls: solve_calls.clone(),
                },
                vec![LegacyValue::F64(input)],
            )
            .unwrap();

            let output_cell = result
                .reactive_cell_ids()
                .into_iter()
                .next()
                .expect("initialized compiler result should expose an output cell");
            assert!(result.reactive_cell_ids().contains(&output_cell));
            assert_eq!(solve_calls.load(Ordering::SeqCst), 1);
            let plan_borrow = plan.borrow();
            let node = plan_borrow.node(0).unwrap();
            assert_eq!(plan_borrow.len(), 1);
            assert!(node.inputs.iter().any(|dependency| {
                dependency.cell == input_cell && dependency.kind == ReactiveDependencyKind::Reactive
            }));
            assert_eq!(plan_borrow.reactive_consumers_for(input_cell), &[0]);
            assert!(plan_borrow.sampled_consumers_for(input_cell).is_empty());
            assert!(node.outputs.contains(&output_cell));
            assert!(
                !node
                    .inputs
                    .iter()
                    .any(|dependency| dependency.cell == output_cell)
            );
        }

        #[derive(Debug, Clone)]
        struct DeferredNativeSolveError;

        impl MechErrorKind for DeferredNativeSolveError {
            fn name(&self) -> &str {
                "DeferredNativeSolveError"
            }
            fn message(&self) -> String {
                "deferred native solve error".to_string()
            }
        }

        struct DeferredNativeSolveCompiler {
            solve_result_calls: Arc<std::sync::atomic::AtomicUsize>,
        }
        struct DeferredNativeSolveFunction {
            output: LegacyValue,
            solve_result_calls: Arc<std::sync::atomic::AtomicUsize>,
        }

        impl FunctionSpecializer for DeferredNativeSolveCompiler {
            fn specialize(&self, _arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
                Ok(Box::new(DeferredNativeSolveFunction {
                    output: LegacyValue::F64(Ref::new(2.0)),
                    solve_result_calls: self.solve_result_calls.clone(),
                }))
            }
        }
        impl MechFunctionImpl for DeferredNativeSolveFunction {
            fn solve_result(&self) -> MResult<()> {
                self.solve_result_calls
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Err(MechError::new(DeferredNativeSolveError, None))
            }
            fn out(&self) -> LegacyValue {
                self.output.clone()
            }
            fn to_string(&self) -> String {
                "deferred-native-solve".to_string()
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }
        #[cfg(feature = "compiler")]
        impl MechFunctionCompiler for DeferredNativeSolveFunction {
            fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                Ok(0)
            }
        }

        #[test]
        fn native_registration_defers_solve_result_errors() {
            let interpreter = Interpreter::new(0, 100);
            let mut services = NoMechExecutionServices;
            let execution = InterpreterExecution::new(&interpreter, &mut services);
            let input = Ref::new(1.0);
            let input_cell = ReactiveCellId::new(input.id());
            let arguments = vec![LegacyValue::F64(input)];
            let solve_result_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let plan = interpreter.plan();
            plan.push_activation_registration_scope(vec![input_cell]);
            let result = execute_function_specializer(
                Arc::new(DeferredNativeSolveCompiler {
                    solve_result_calls: solve_result_calls.clone(),
                }),
                &arguments,
                &execution,
            );
            plan.pop_activation_registration_scope();

            assert!(result.is_ok());
            assert_eq!(
                solve_result_calls.load(std::sync::atomic::Ordering::SeqCst),
                0
            );
            let plan = plan.borrow();
            assert!(
                plan.nodes
                    .last()
                    .unwrap()
                    .inputs
                    .iter()
                    .any(|dependency| dependency.cell == input_cell)
            );
            assert!(
                plan.nodes
                    .last()
                    .unwrap()
                    .function
                    .solve_result()
                    .unwrap_err()
                    .kind_name()
                    .contains("DeferredNativeSolveError")
            );
            assert_eq!(
                solve_result_calls.load(std::sync::atomic::Ordering::SeqCst),
                1
            );
            assert_eq!(plan.len(), 1);
        }

        #[test]
        fn native_function_registration_records_operand_cells() {
            let interpreter = Interpreter::new(0, 100);
            let mut services = NoMechExecutionServices;
            let execution = InterpreterExecution::new(&interpreter, &mut services);
            let input = Ref::new(1.0);
            let input_cell = ReactiveCellId::new(input.id());
            let arguments = vec![LegacyValue::F64(input)];

            let result = execute_function_specializer(
                Arc::new(NativeDependencyTestCompiler),
                &arguments,
                &execution,
            )
            .unwrap();

            let output_cell = match result {
                LegacyValue::F64(output) => ReactiveCellId::new(output.id()),
                other => panic!("expected f64 output, found {:?}", other),
            };

            let plan = interpreter.plan();
            let plan_borrow = plan.borrow();
            let node = plan_borrow
                .nodes
                .iter()
                .find(|node| !node.inputs.is_empty())
                .expect("native compiler path should register indexed inputs");

            assert!(node.inputs.iter().any(|dependency| {
                dependency.cell == input_cell && dependency.kind == ReactiveDependencyKind::Reactive
            }));
            assert_eq!(plan_borrow.reactive_consumers_for(input_cell), &[node.id],);
            assert!(node.outputs.contains(&output_cell));
        }
    }

    #[cfg(all(test, feature = "functions", feature = "f64"))]
    mod native_initialization_failure_tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct FailingInitializationCompiler {
            solve_result_calls: Arc<AtomicUsize>,
            preserved_initialization_calls: Arc<AtomicUsize>,
            initial_solve_policy: InitialSolvePolicy,
        }

        struct FailingInitializationFunction {
            solve_result_calls: Arc<AtomicUsize>,
            preserved_initialization_calls: Arc<AtomicUsize>,
            output: Ref<f64>,
            initial_solve_policy: InitialSolvePolicy,
        }

        impl FunctionSpecializer for FailingInitializationCompiler {
            fn specialize(&self, _arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
                Ok(Box::new(FailingInitializationFunction {
                    solve_result_calls: self.solve_result_calls.clone(),
                    preserved_initialization_calls: self.preserved_initialization_calls.clone(),
                    output: Ref::new(123.0),
                    initial_solve_policy: self.initial_solve_policy,
                }))
            }
        }

        impl MechFunctionImpl for FailingInitializationFunction {
            fn solve_result(&self) -> MResult<()> {
                self.solve_result_calls.fetch_add(1, Ordering::SeqCst);
                Err(MechError::new(
                    GenericError {
                        msg: "test native initialization failed".to_string(),
                    },
                    None,
                ))
            }

            fn initial_solve_policy(&self) -> InitialSolvePolicy {
                self.initial_solve_policy
            }

            fn initialize_preserved_output_with(
                &self,
                _services: &mut dyn MechExecutionServices,
            ) -> MResult<()> {
                self.preserved_initialization_calls
                    .fetch_add(1, Ordering::SeqCst);
                Ok(())
            }

            fn out(&self) -> LegacyValue {
                LegacyValue::F64(self.output.clone())
            }

            fn to_string(&self) -> String {
                "failing-initialization-test".to_string()
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }

        #[cfg(feature = "compiler")]
        impl MechFunctionCompiler for FailingInitializationFunction {
            fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                panic!("failing initialization test function must not be bytecode compiled")
            }
        }

        fn failing_compiler(solve_result_calls: Arc<AtomicUsize>) -> Arc<dyn FunctionSpecializer> {
            Arc::new(FailingInitializationCompiler {
                solve_result_calls,
                preserved_initialization_calls: Arc::new(AtomicUsize::new(0)),
                initial_solve_policy: InitialSolvePolicy::Solve,
            })
        }

        #[test]
        fn eager_initialization_uses_solve_result() {
            let interpreter = Interpreter::new(0, 100);
            let mut services = NoMechExecutionServices;
            let execution = InterpreterExecution::new(&interpreter, &mut services);
            let arguments = vec![LegacyValue::F64(Ref::new(1.0))];
            let solve_result_calls = Arc::new(AtomicUsize::new(0));
            let plan_len = interpreter.plan().len();

            let error = execute_function_specializer(
                failing_compiler(solve_result_calls.clone()),
                &arguments,
                &execution,
            )
            .expect_err("eager initialization must return the native solve error");

            assert!(
                error
                    .full_chain_message()
                    .contains("test native initialization failed")
            );
            assert_eq!(solve_result_calls.load(Ordering::SeqCst), 1);
            assert_eq!(interpreter.plan().len(), plan_len);
        }

        #[test]
        fn activation_registration_defers_initialization_solving() {
            let interpreter = Interpreter::new(0, 100);
            let mut services = NoMechExecutionServices;
            let execution = InterpreterExecution::new(&interpreter, &mut services);
            let input = Ref::new(1.0);
            let arguments = vec![LegacyValue::F64(input.clone())];
            let solve_result_calls = Arc::new(AtomicUsize::new(0));
            let plan = interpreter.plan();
            let plan_len = plan.len();

            plan.push_activation_registration_scope(vec![ReactiveCellId::new(input.id())]);
            let result = execute_function_specializer(
                failing_compiler(solve_result_calls.clone()),
                &arguments,
                &execution,
            );
            plan.pop_activation_registration_scope();

            assert!(result.is_ok());
            assert_eq!(solve_result_calls.load(Ordering::SeqCst), 0);
            assert_eq!(plan.len(), plan_len + 1);
        }

        #[test]
        fn preserve_specialized_output_skips_initial_solve_and_registers_once() {
            let interpreter = Interpreter::new(0, 100);
            let mut services = NoMechExecutionServices;
            let execution = InterpreterExecution::new(&interpreter, &mut services);
            let arguments = vec![LegacyValue::F64(Ref::new(1.0))];
            let solve_result_calls = Arc::new(AtomicUsize::new(0));
            let preserved_initialization_calls = Arc::new(AtomicUsize::new(0));
            let plan = interpreter.plan();
            let plan_len = plan.len();

            let result = execute_function_specializer(
                Arc::new(FailingInitializationCompiler {
                    solve_result_calls: solve_result_calls.clone(),
                    preserved_initialization_calls: preserved_initialization_calls.clone(),
                    initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
                }),
                &arguments,
                &execution,
            )
            .expect("the planned output must be preserved without calling solve_result");

            assert!(matches!(result, LegacyValue::F64(_)));
            assert_eq!(solve_result_calls.load(Ordering::SeqCst), 0);
            assert_eq!(preserved_initialization_calls.load(Ordering::SeqCst), 1);
            assert_eq!(plan.len(), plan_len + 1);
        }
    }

    #[cfg(all(
        test,
        feature = "source",
        feature = "functions",
        feature = "kind_annotation",
        feature = "matrix",
        feature = "table",
        feature = "tuple",
        feature = "f64"
    ))]
    mod outer_lift_tests {
        use super::*;

        fn interpret(source: &str) -> LegacyValue {
            let tree = mech_syntax::parser::parse(source).unwrap();
            let mut interpreter = Interpreter::with_function_catalog(
                0,
                10_000,
                crate::test_support::catalog::function_catalog(),
            );
            interpreter.interpret(&tree).unwrap()
        }

        #[test]
        fn user_function_lifts_multiple_arguments_and_outputs() {
            let result = interpret(
                r#"sum-product(left<f64>, right<f64>) = (sum<f64>, product<f64>) :=
  sum := left + right
  product := left * right.

sum-product([1.0, 2.0, 3.0], [4.0, 5.0, 6.0])"#,
            );

            assert_eq!(
                result,
                LegacyValue::Tuple(Ref::new(MechTuple::from_vec(vec![
                    LegacyValue::MatrixF64(Matrix::from_vec(vec![5.0, 7.0, 9.0], 1, 3)),
                    LegacyValue::MatrixF64(Matrix::from_vec(vec![4.0, 10.0, 18.0], 1, 3)),
                ])))
            );
        }

        #[test]
        fn user_function_lift_distinguishes_inner_fixed_matrices() {
            let result = interpret(
                r#"copy-state(state<[f64]:2,1>) = next-state<[f64]:2,1> :=
  next-state := state.

batch := |state<*> scale<f64>|
  | [1.0; 2.0] 2.0 |
  | [3.0; 4.0] 3.0 |

copy-state(batch.state)"#,
            );

            assert_eq!(
                result,
                LegacyValue::MatrixValue(Matrix::from_vec(
                    vec![
                        LegacyValue::MatrixF64(Matrix::from_vec(vec![1.0, 2.0], 2, 1)),
                        LegacyValue::MatrixF64(Matrix::from_vec(vec![3.0, 4.0], 2, 1)),
                    ],
                    2,
                    1,
                ))
            );
        }
    }
} // mod source_only

#[cfg(feature = "source")]
pub use source_only::*;
pub mod catalog;
pub mod environment;
pub mod extensions;
#[cfg(feature = "program")]
pub mod external;
#[cfg(all(feature = "source", feature = "functions"))]
pub mod module;
#[cfg(all(feature = "source", feature = "native"))]
pub mod native;
pub mod resolver;

pub use catalog::*;
pub use environment::*;
pub use extensions::*;
#[cfg(feature = "program")]
pub use external::*;
#[cfg(all(feature = "source", feature = "functions"))]
pub use module::*;
#[cfg(all(feature = "source", feature = "native"))]
pub use native::*;
pub use resolver::*;
