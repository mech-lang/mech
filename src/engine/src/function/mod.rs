#[cfg(feature = "semantic-compiler")]
pub use crate::expressions::function_call;
#[cfg(feature = "trace")]
use crate::tracing::{format_trace, summarize_function_pattern};
#[cfg(feature = "semantic-compiler")]
use crate::*;
#[cfg(all(
    feature = "semantic-compiler",
    feature = "kind_annotation",
    feature = "enum"
))]
use std::collections::HashSet;

// Functions
// ============================================================================

// Registers a user-written function so it can be called by name later.
// Hashes the name to a u64 id used as the lookup key throughout the runtime.
#[cfg(feature = "semantic-compiler")]
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

    pub fn execute_bound_specialized_function(
        specialized: SpecializedFunction,
        _input_arg_values: &[SpecializationInput],
        p: &InterpreterExecution<'_>,
    ) -> MResult<ValueCell> {
        let plan = p.plan();
        let instance = specialized.instance();
        let implementation = instance.implementation();
        trace_println!(
            p,
            "{}",
            format_trace(
                "arm",
                format!(
                    "selected {} args=[{}]",
                    implementation
                        .to_string()
                        .lines()
                        .next()
                        .unwrap_or("<unknown-arm>"),
                    _input_arg_values
                        .iter()
                        .map(|cell| format!("{cell:?}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        );
        solve_specialized_initial_output(implementation, &plan, p)?;
        let result = instance.output().clone();
        trace_println!(p, "{}", format_trace("arm", format!("result {result:?}")));
        plan.register_specialized(specialized)?;
        Ok(result)
    }

    pub fn execute_catalog_operation_with_registration_arguments(
        p: &InterpreterExecution<'_>,
        plan: &Plan,
        canonical_name: &str,
        compile_arguments: Vec<ValueCell>,
        _registration_arguments: Vec<ValueCell>,
    ) -> MResult<ValueCell> {
        let operation = OperationId::from_name(canonical_name);
        let invocation = SpecializationInvocation::from_cells(compile_arguments.into_boxed_slice());
        let specialized =
            p.specialize_visible_invocation_named(operation, Some(canonical_name), &invocation)?;
        execute_function_instance(p, plan, specialized)
    }

    pub(crate) fn execute_function_instance(
        p: &InterpreterExecution<'_>,
        plan: &Plan,
        specialized: SpecializedFunction,
    ) -> MResult<ValueCell> {
        let instance = specialized.instance();
        solve_specialized_initial_output(instance.implementation(), plan, p)?;
        let output = instance.output().clone();
        plan.register_specialized(specialized)?;
        Ok(output)
    }

    #[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
    pub(crate) fn execute_catalog_operation(
        p: &InterpreterExecution<'_>,
        plan: &Plan,
        canonical_name: &str,
        arguments: Vec<ValueCell>,
    ) -> MResult<ValueCell> {
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
        input_arg_values: &[ValueCell],
        p: &InterpreterExecution<'_>,
    ) -> MResult<ValueCell> {
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
        #[cfg(all(feature = "matrix", feature = "kind_annotation"))]
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
                    input_arg_values
                        .iter()
                        .map(|value| format!("{value:?}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        );

        // Choose execution strategy: match-arm body vs. plain statement body.
        let output = if !fxn_def.code.match_arms.is_empty() {
            // Match-arm body: loop to support tail-call optimisation. Each iteration
            // opens a fresh scope, binds the current arguments, runs the arms, then
            // either returns the result or loops with a new argument set.
            let mut current_args: Vec<ValueCell> = input_arg_values.to_vec();
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
                        format!("exit  {} => {}", fxn_def.name, format!("{value:?}"))
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
        Return(ValueCell),
        TailCall(Vec<ValueCell>),
    }

    // If the function is single-input / single-output with matching scalar kinds,
    // and the actual argument is a matrix, run the function on each element and
    // reassemble the result into a matrix of the same shape.
    // Returns None if any condition for broadcasting isn't met, so the caller can
    // fall through to normal execution.
    #[cfg(all(feature = "matrix", feature = "kind_annotation"))]
    fn try_broadcast_user_function(
        fxn_def: &FunctionDefinition,
        input_arg_values: &[ValueCell],
        p: &InterpreterExecution<'_>,
    ) -> MResult<Option<ValueCell>> {
        if input_arg_values.len() != 1
            || fxn_def.code.output.len() != 1
            || fxn_def.code.input.len() != 1
        {
            return Ok(None);
        }

        let source = &input_arg_values[0];
        let SchemaBody::Matrix { dimensions, .. } = source.closed_schema_body()? else {
            return Ok(None);
        };

        let input_kind =
            crate::structures::schema_body_from_kind(&fxn_def.code.input[0].kind.kind, p)?;
        let output_kind =
            crate::structures::schema_body_from_kind(&fxn_def.code.output[0].kind.kind, p)?;

        // Only broadcast when input and output kinds are the same scalar kind.
        // If the input is already a matrix kind, don't recurse.
        if input_kind != output_kind || matches!(input_kind, SchemaBody::Matrix { .. }) {
            return Ok(None);
        }

        let Some(elements) = crate::patterns::matrix_like_values(source)? else {
            return Ok(None);
        };

        // Apply the function element-wise, then reassemble into the original shape.
        let mut outputs = Vec::with_capacity(elements.len());
        for element in elements {
            outputs.push(execute_user_function(fxn_def, &[element], p)?);
        }
        let [
            DimensionExpr::Constant(rows),
            DimensionExpr::Constant(columns),
        ] = dimensions.as_ref()
        else {
            unreachable!("closed matrix schema has concrete dimensions")
        };
        Ok(Some(ValueCell::dynamic_matrix_from_cells(
            *rows as usize,
            *columns as usize,
            &outputs,
        )?))
    }

    // Tries each match arm in order against the current arguments. Handles:
    //   - enum exhaustiveness checking (kind_annotation + enum features)
    //   - tail-call detection (arm body is a recursive call with same arity)
    //   - output kind coercion
    // Returns an error if no arm matched.
    fn execute_function_match_arms(
        fxn_def: &FunctionDefinition,
        input_arg_values: &[ValueCell],
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
                    let input_schema =
                        crate::structures::schema_body_from_kind(&kind_annotation_node.kind, p)?;
                    if let SchemaBody::Enum { variants, .. } = input_schema {
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
                        let all_covered = variants
                            .iter()
                            .all(|variant| covered_variants.contains(&hash_str(&variant.name)));
                        if !all_covered {
                            // Build a readable list of the missing variant patterns.
                            let missing_patterns = variants
                                .iter()
                                .filter(|variant| {
                                    !covered_variants.contains(&hash_str(&variant.name))
                                })
                                .map(|variant| {
                                    if variant.payload.is_some() {
                                        format!(":{}(…)", variant.name)
                                    } else {
                                        format!(":{}", variant.name)
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

        // Try each arm in source order; the first one whose pattern matches wins.
        for indexed_arm in fxn_def.code.match_arms.iter().enumerate() {
            #[cfg(feature = "trace")]
            let (arm_idx, arm) = indexed_arm;
            #[cfg(not(feature = "trace"))]
            let (_, arm) = indexed_arm;
            let mut env = Environment::new();
            let matched = crate::patterns::pattern_matches_arguments(
                &arm.pattern,
                input_arg_values,
                &mut env,
                p,
            )?;
            trace_println!(p, "{}", {
                let args_summary = input_arg_values
                    .iter()
                    .map(|value| format!("{value:?}"))
                    .collect::<Vec<_>>()
                    .join(", ");
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
                            tail_args.push(expression_cell(arg_expr, Some(&env), p)?);
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
                let coerced = expression_cell(&arm.expression, Some(&env), p)?;
                #[cfg(feature = "kind_annotation")]
                let coerced = coerce_function_output_kind(coerced, fxn_def, p)?;
                trace_println!(
                    p,
                    "{}",
                    format_trace(
                        "match",
                        format!(
                            "arm[{arm_idx}] out  value={} kind={}",
                            format!("{coerced:?}"),
                            format!("{:?}", coerced.representation())
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
        value: ValueCell,
        fxn_def: &FunctionDefinition,
        p: &InterpreterExecution<'_>,
    ) -> MResult<ValueCell> {
        let Some(output_arg) = fxn_def.code.output.first() else {
            return Ok(value);
        };
        coerce_value_to_declared_kind(value, &output_arg.kind, p)
            .map_err(|error| error.with_tokens(output_arg.tokens()))
    }

    #[cfg(feature = "kind_annotation")]
    fn coerce_value_to_declared_kind(
        value: ValueCell,
        output_kind_annotation: &KindAnnotation,
        p: &InterpreterExecution<'_>,
    ) -> MResult<ValueCell> {
        let target_schema =
            crate::structures::schema_body_from_kind(&output_kind_annotation.kind, p)?;
        if value.closed_schema_body()? == target_schema {
            return Ok(value);
        }
        crate::literals::convert_cell_implicitly_reactively(value, target_schema, p)
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
            state_brrw.user_function_scope_depth += 1;
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
            debug_assert!(state_brrw.user_function_scope_depth > 0);
            state_brrw.user_function_scope_depth -= 1;
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
        input_arg_values: &[ValueCell],
        p: &InterpreterExecution<'_>,
    ) -> MResult<()> {
        for (input_argument, input_value) in fxn_def.input.iter().zip(input_arg_values.iter()) {
            let arg_id = input_argument.0;
            #[cfg(feature = "kind_annotation")]
            let input_kind_annotation = &input_argument.1;
            // Look up the human-readable argument name for error messages.
            let arg_name = fxn_def
                .code
                .input
                .iter()
                .find(|arg| arg.name.hash() == *arg_id)
                .map(|arg| arg.name.to_string())
                .unwrap_or_else(|| arg_id.to_string());

            #[cfg(feature = "kind_annotation")]
            let bound_value = {
                let target_schema =
                    crate::structures::schema_body_from_kind(&input_kind_annotation.kind, p)?;
                if input_value.closed_schema_body()? == target_schema {
                    input_value.clone()
                } else {
                    crate::literals::convert_cell_implicitly_reactively(
                        input_value.clone(),
                        target_schema,
                        p,
                    )
                    .map_err(|error| error.with_tokens(input_kind_annotation.tokens()))?
                }
            };
            #[cfg(not(feature = "kind_annotation"))]
            let bound_value = input_value.clone();
            #[cfg(feature = "subscript_formula")]
            if current_string_access_expression_live(p)
                || string_access_input_is_live(input_value, p)
            {
                mark_string_access_value_live(p, &bound_value);
            }
            p.state
                .borrow()
                .save_symbol(*arg_id, arg_name, bound_value, false);
        }
        Ok(())
    }

    // Reads each declared output variable out of the local symbol table and
    // returns them as a single Value. Multiple outputs are wrapped in a Tuple;
    // a single output is returned directly; zero outputs return Empty.
    fn collect_function_output(
        p: &InterpreterExecution<'_>,
        fxn_def: &FunctionDefinition,
    ) -> MResult<ValueCell> {
        let symbols = p.symbols();
        let symbols_brrw = symbols.borrow();
        let mut outputs = vec![];

        for output_arg in &fxn_def.code.output {
            let output_id = output_arg.name.hash();
            match symbols_brrw.get(output_id) {
                Some(cell) => outputs.push(cell),
                None => {
                    return Err(
                        MechError::new(FunctionOutputUndefinedError { output_id }, None)
                            .with_compiler_loc()
                            .with_tokens(output_arg.tokens()),
                    );
                }
            }
        }
        drop(symbols_brrw);

        #[cfg(feature = "kind_annotation")]
        for (output, output_arg) in outputs.iter_mut().zip(&fxn_def.code.output) {
            *output = coerce_value_to_declared_kind(output.clone(), &output_arg.kind, p)
                .map_err(|error| error.with_tokens(output_arg.tokens()))?;
        }

        match outputs.len() {
            0 => Ok(ValueCell::unit()),
            1 => Ok(outputs.remove(0)),
            #[cfg(feature = "tuple")]
            _ => ValueCell::tuple_from_cells(&outputs),
            #[cfg(not(feature = "tuple"))]
            _ => {
                return Err(MechError::new(FeatureNotEnabledError, None)
                    .with_compiler_loc()
                    .with_tokens(fxn_def.code.name.tokens()));
            }
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
        pub expected: String,
        pub found: String,
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
} // mod source_only

#[cfg(feature = "semantic-compiler")]
pub use source_only::*;
#[path = "catalog.rs"]
pub(crate) mod engine_catalog;
pub mod environment;
pub mod extensions;
#[cfg(feature = "program")]
pub mod external;
#[cfg(all(feature = "semantic-compiler", feature = "functions"))]
pub mod module;
#[cfg(all(feature = "semantic-compiler", feature = "native"))]
pub mod native;
pub mod resolver;

pub use engine_catalog::*;
pub use environment::*;
pub use extensions::*;
#[cfg(feature = "program")]
pub use external::*;
#[cfg(all(feature = "semantic-compiler", feature = "functions"))]
pub use module::*;
#[cfg(all(feature = "semantic-compiler", feature = "native"))]
pub use native::*;
pub use resolver::*;
