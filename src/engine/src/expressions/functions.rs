use super::{Environment, expression};
#[cfg(feature = "subscript_formula")]
use super::{
    current_string_access_expression_live, mark_current_string_access_expression_live,
    mark_string_access_value_live, string_access_input_is_live,
};
#[cfg(feature = "trace")]
use crate::format_trace;
use crate::{
    FunctionCall, FunctionDefinition, FunctionExtensionEntry, FunctionResolver,
    FunctionSpecializerEntry, InterpreterExecution, MResult, ResolvedNamedFunction,
    SpecializationInput, ValueCell, execute_bound_specialized_function,
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
) -> MResult<Vec<SpecializationInput>> {
    fxn_call
        .args
        .iter()
        .map(|(_, argument)| expression(argument, env, p))
        .collect()
}

// Dispatches a named function through the program's unified resolver. User
// definitions have precedence over the current catalog or extension binding.
pub fn function_call(
    fxn_call: &FunctionCall,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
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
                OwnedResolvedNamedFunction::User(definition.clone())
            }
            Ok(ResolvedNamedFunction::Catalog(entry)) => {
                OwnedResolvedNamedFunction::Catalog(entry.clone())
            }
            Ok(ResolvedNamedFunction::Extension(entry)) => {
                OwnedResolvedNamedFunction::Extension(entry.clone())
            }
            Err(error) => return Err(error.with_tokens(fxn_call.name.tokens())),
        }
    };

    let input_arg_values = evaluate_arguments(fxn_call, env, p)?;
    match resolved {
        OwnedResolvedNamedFunction::User(definition) => {
            let user_inputs = input_arg_values
                .iter()
                .map(|input| input.cell().cloned())
                .collect::<MResult<Vec<_>>>()?;
            #[cfg(feature = "subscript_formula")]
            let output_is_live = current_string_access_expression_live(p)
                || input_arg_values
                    .iter()
                    .filter_map(|input| input.cell().ok())
                    .any(|value| string_access_input_is_live(value, p));
            let output = crate::function::execute_user_function(&definition, &user_inputs, p)?;
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
                        input_arg_values
                            .iter()
                            .map(|input| format!("{input:?}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                )
            );
            let invocation = mech_core::SpecializationInvocation::new(
                input_arg_values.clone().into_boxed_slice(),
            );
            let specialized = p
                .specialize_visible_invocation_named(
                    entry.operation,
                    Some(&entry.canonical_name),
                    &invocation,
                )
                .map_err(|error| error.with_tokens(fxn_call.name.tokens()))?;
            execute_bound_specialized_function(specialized, &input_arg_values, p)
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
                        input_arg_values
                            .iter()
                            .map(|input| format!("{input:?}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                )
            );
            let invocation = mech_core::SpecializationInvocation::new(
                input_arg_values.clone().into_boxed_slice(),
            );
            let mut context = mech_core::SpecializationContext::for_syntax_directed_invocation(
                &invocation,
                Some(p.function_catalog()),
                mech_core::OperationId::from_raw(entry.id.raw()),
                entry.canonical_name.clone(),
            )?;
            let specialized = entry
                .specializer
                .specialize_invocation(&invocation, &mut context)?;
            execute_bound_specialized_function(specialized, &input_arg_values, p)
        }
    }
}
