use super::{Environment, expression};
#[cfg(feature = "subscript_formula")]
use super::{
    current_string_access_expression_live, mark_current_string_access_expression_live,
    mark_string_access_value_live, string_access_input_is_live,
};
use crate::{
    FunctionCall, FunctionDefinition, FunctionExtensionEntry, FunctionOperationNotVisible,
    FunctionOperationUnavailable, FunctionResolver, FunctionSpecializerEntry, InterpreterExecution,
    MResult, MechError, MissingFunctionError, OperationId, ResolvedNamedFunction, Value,
    execute_native_function_compiler, execute_specialized_function, format_trace,
    format_trace_args,
};

enum OwnedResolvedNamedFunction {
    User(FunctionDefinition),
    Catalog(FunctionSpecializerEntry),
    Extension(FunctionExtensionEntry),
}

fn evaluate_arguments(
    fxn_call: &FunctionCall,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<Vec<Value>> {
    fxn_call
        .args
        .iter()
        .map(|(_, argument)| expression(argument, env, p))
        .collect()
}

// Dispatches a function call to whichever implementation is available:
// user-defined functions first, then built-in functions, then native compiled
// functions. Returns an error if the name is not found in any registry.
pub fn function_call(
    fxn_call: &FunctionCall,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<Value> {
    let functions = p.functions();
    let fxn_name_id = fxn_call.name.hash();
    let fxn_name = fxn_call.name.to_string();

    let resolved = {
        let state = p.state.borrow();
        let resolver = FunctionResolver::new(
            p.function_catalog(),
            &state.function_environment,
            &state.function_extensions,
            &state.user_functions,
        );
        match resolver.resolve_named(&fxn_name) {
            Ok(ResolvedNamedFunction::User(definition)) => {
                Some(OwnedResolvedNamedFunction::User(definition.clone()))
            }
            Ok(ResolvedNamedFunction::Catalog(entry)) => {
                Some(OwnedResolvedNamedFunction::Catalog(entry.clone()))
            }
            Ok(ResolvedNamedFunction::Extension(entry)) => {
                Some(OwnedResolvedNamedFunction::Extension(entry.clone()))
            }
            Err(error) if error.kind_name() == "MissingFunction" => {
                let operation = OperationId::from_name(&fxn_name);
                if p.legacy_function_boundary()
                    .owns_named_operation(operation, &fxn_name)
                {
                    let boundary_error = if p.function_catalog().specializer(operation).is_some() {
                        MechError::new(
                            FunctionOperationNotVisible {
                                operation,
                                canonical_name: Some(fxn_name.clone()),
                            },
                            None,
                        )
                    } else {
                        MechError::new(
                            FunctionOperationUnavailable {
                                operation,
                                canonical_name: Some(fxn_name.clone()),
                            },
                            None,
                        )
                    };
                    return Err(boundary_error
                        .with_compiler_loc()
                        .with_tokens(fxn_call.name.tokens()));
                }
                None
            }
            Err(error) => return Err(error.with_tokens(fxn_call.name.tokens())),
        }
    };

    if let Some(resolved) = resolved {
        let input_arg_values = evaluate_arguments(fxn_call, env, p)?;
        return match resolved {
            OwnedResolvedNamedFunction::User(definition) => {
                #[cfg(feature = "subscript_formula")]
                let output_is_live = current_string_access_expression_live(p)
                    || input_arg_values
                        .iter()
                        .any(|value| string_access_input_is_live(value, p));
                let output =
                    crate::functions::execute_user_function(&definition, &input_arg_values, p)?;
                #[cfg(feature = "subscript_formula")]
                if output_is_live {
                    mark_current_string_access_expression_live(p);
                    mark_string_access_value_live(p, &output);
                }
                Ok(output)
            }
            OwnedResolvedNamedFunction::Catalog(entry) => {
                trace_println!(
                    p,
                    "{}",
                    format_trace(
                        "fn",
                        format!(
                            "catalog {}({})",
                            fxn_name,
                            format_trace_args(&input_arg_values)
                        ),
                    )
                );
                let function = entry.specializer.specialize(&input_arg_values)?;
                execute_specialized_function(function, &input_arg_values, p)
            }
            OwnedResolvedNamedFunction::Extension(entry) => {
                trace_println!(
                    p,
                    "{}",
                    format_trace(
                        "fn",
                        format!(
                            "extension {}({})",
                            fxn_name,
                            format_trace_args(&input_arg_values)
                        ),
                    )
                );
                let function = entry.specializer.specialize(&input_arg_values)?;
                execute_specialized_function(function, &input_arg_values, p)
            }
        };
    }

    // Pre-compiled built-in functions.
    if { functions.borrow().functions.contains_key(&fxn_name_id) } {
        todo!();
    }

    // Native function compiler: the compiler picks a concrete implementation
    // based on the runtime argument types, then we execute it immediately.
    let fxn_compiler = {
        functions
            .borrow()
            .function_compilers
            .get(&fxn_name_id)
            .cloned()
    };
    match fxn_compiler {
        Some(fxn_compiler) => {
            let input_arg_values = evaluate_arguments(fxn_call, env, p)?;
            trace_println!(
                p,
                "{}",
                format_trace(
                    "fn",
                    format!(
                        "native {}({})",
                        fxn_name,
                        format_trace_args(&input_arg_values)
                    ),
                )
            );
            execute_native_function_compiler(fxn_compiler, &input_arg_values, p)
        }
        // No implementation found under this name at all.
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
