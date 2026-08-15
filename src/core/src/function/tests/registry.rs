use super::super::{
    FunctionArgs, FunctionDefinition, FunctionValueRepresentation, GuardFunctionSafety,
    MechFunction, RuntimeFunctionSignature, UserFunctionTable,
};
#[cfg(feature = "f64")]
use super::support::scalar;
use crate::{
    FunctionDefine, FunctionSpecializer, LegacyValue, MResult, hash_str,
    internal_pattern_value_identifier,
};

fn user_definition(name: &str) -> FunctionDefinition {
    FunctionDefinition::new(
        hash_str(name),
        name.to_string(),
        FunctionDefine {
            name: internal_pattern_value_identifier(name),
            input: Vec::new(),
            output: Vec::new(),
            statements: Vec::new(),
            match_arms: Vec::new(),
        },
    )
}

struct DefaultTestSpecializer;

impl FunctionSpecializer for DefaultTestSpecializer {
    fn specialize(&self, _arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        unreachable!("safety metadata test must not specialize the function")
    }
}

#[test]
fn function_specializer_guard_safety_defaults_to_unsupported() {
    let specializer = DefaultTestSpecializer;

    assert_eq!(specializer.guard_safety(), GuardFunctionSafety::Unsupported);
}

#[cfg(feature = "f64")]
#[test]
fn function_args_returns_only_inputs() {
    let (out, _) = scalar(0.0);
    let (a, _) = scalar(1.0);
    let (b, _) = scalar(2.0);
    let (c, _) = scalar(3.0);
    let (d, _) = scalar(4.0);

    assert_eq!(
        FunctionArgs::Nullary(out.clone()).input_values(),
        Vec::<LegacyValue>::new()
    );
    assert_eq!(
        FunctionArgs::Unary(out.clone(), a.clone()).input_values(),
        vec![a.clone()]
    );
    assert_eq!(
        FunctionArgs::Binary(out.clone(), a.clone(), b.clone()).input_values(),
        vec![a.clone(), b.clone()],
    );
    assert_eq!(
        FunctionArgs::Ternary(out.clone(), a.clone(), b.clone(), c.clone()).input_values(),
        vec![a.clone(), b.clone(), c.clone()],
    );
    assert_eq!(
        FunctionArgs::Quaternary(out.clone(), a.clone(), b.clone(), c.clone(), d.clone(),)
            .input_values(),
        vec![a.clone(), b.clone(), c.clone(), d.clone()],
    );
    assert_eq!(
        FunctionArgs::Variadic(out, vec![a.clone(), b.clone(), c.clone(), d.clone()])
            .input_values(),
        vec![a, b, c, d],
    );
}

#[cfg(feature = "f64")]
#[test]
fn variadic_signatures_normalize_bytecode_instruction_arities() {
    let (out, _) = scalar(0.0);
    let (a, _) = scalar(1.0);
    let signature = RuntimeFunctionSignature::variadic(
        FunctionValueRepresentation::F64,
        FunctionValueRepresentation::F64,
    );

    assert!(matches!(
        FunctionArgs::Nullary(out.clone()).normalize_for_signature(signature),
        FunctionArgs::Variadic(_, arguments) if arguments.is_empty()
    ));
    assert!(matches!(
        FunctionArgs::Unary(out, a).normalize_for_signature(signature),
        FunctionArgs::Variadic(_, arguments) if arguments.len() == 1
    ));
}

#[test]
fn user_function_table_replaces_only_the_exact_same_name() {
    let mut definitions = UserFunctionTable::default();
    let first = user_definition("local/read");
    let first_out = first.out.clone();
    definitions.insert_or_replace(first).unwrap();

    let replacement = user_definition("local/read");
    let replacement_out = replacement.out.clone();
    let replaced = definitions.insert_or_replace(replacement).unwrap().unwrap();

    assert_eq!(replaced.out.addr(), first_out.addr());
    assert_eq!(
        definitions.resolve_name("local/read").unwrap().out.addr(),
        replacement_out.addr(),
    );
    assert_eq!(definitions.definitions().len(), 1);
}

#[test]
fn user_function_table_rejects_a_distinct_name_at_one_forced_id() {
    let incoming = user_definition("second");
    let id = incoming.id;
    let mut existing = user_definition("first");
    existing.id = id;
    let mut definitions = UserFunctionTable::default();
    definitions.definitions.insert(id, existing);

    let error = definitions.insert_or_replace(incoming).unwrap_err();

    assert_eq!(error.kind_name(), "UserFunctionIdCollision");
    assert_eq!(definitions.definitions.get(&id).unwrap().name, "first");
}

#[test]
fn user_function_table_name_resolution_is_exact() {
    let mut definitions = UserFunctionTable::default();
    definitions
        .insert_or_replace(user_definition("module/item"))
        .unwrap();

    assert!(definitions.resolve_name("module/item").is_some());
    assert!(definitions.resolve_name("item").is_none());
    assert_eq!(definitions.len(), 1);
    assert!(!definitions.is_empty());
    definitions.clear();
    assert!(definitions.is_empty());
}
