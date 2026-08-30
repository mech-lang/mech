use super::super::{FunctionDefinition, UserFunctionTable};
use crate::{FunctionDefine, hash_str, internal_pattern_value_identifier};

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

#[test]
fn user_function_table_replaces_only_the_exact_same_name() {
    let mut definitions = UserFunctionTable::default();
    let first = user_definition("local/read");
    let first_out = first.out.clone();
    definitions.insert_or_replace(first).unwrap();

    let replacement = user_definition("local/read");
    let replacement_out = replacement.out.clone();
    let replaced = definitions.insert_or_replace(replacement).unwrap().unwrap();

    assert!(replaced.out.same_cell(&first_out));
    assert!(
        definitions
            .resolve_name("local/read")
            .unwrap()
            .out
            .same_cell(&replacement_out)
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
fn user_function_table_name_resolution_clear_and_length_are_exact() {
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
