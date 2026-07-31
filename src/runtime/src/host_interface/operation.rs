use std::collections::BTreeSet;

use mech_core::{MResult, MechError};

use crate::InvalidConfigField;

pub fn validate_host_operation_name(operation: &str) -> MResult<()> {
    if operation.is_empty() {
        return invalid("host operation cannot be empty");
    }
    let mut chars = operation.chars();
    let Some(first) = chars.next() else {
        return invalid("host operation cannot be empty");
    };
    if !first.is_ascii_lowercase() {
        return invalid(format!(
            "host operation `{operation}` must start with a lowercase ASCII letter"
        ));
    }
    if !chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-') {
        return invalid(format!(
            "host operation `{operation}` may contain only lowercase ASCII letters, digits, or `-`"
        ));
    }
    Ok(())
}

pub fn validate_host_operations(field: &str, operations: &[String]) -> MResult<()> {
    if operations.is_empty() {
        return invalid(format!("{field} must contain at least one operation"));
    }
    let mut seen = BTreeSet::new();
    for operation in operations {
        validate_host_operation_name(operation)?;
        if !seen.insert(operation.clone()) {
            return invalid(format!("duplicate host operation `{operation}` in {field}"));
        }
    }
    Ok(())
}

fn invalid<T>(message: impl Into<String>) -> MResult<T> {
    Err(MechError::new(InvalidConfigField::new(message), None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_spellable_operation_names() {
        for operation in ["move", "grip", "home", "set-mode", "set-speed2"] {
            validate_host_operation_name(operation).unwrap();
        }
        for operation in ["set_mode", "Move", "moveArm", "move/arm", "move.arm", "move arm", ""] {
            assert!(validate_host_operation_name(operation).is_err(), "{operation} should be invalid");
        }
    }
}
