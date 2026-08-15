use std::collections::BTreeMap;

use crate::{Result, SpecError};

pub(crate) const CONTRACT_PROFILE: &str = "mech-contract-1";

#[derive(Clone, Debug, Eq, PartialEq)]
enum Value {
    Bool(bool),
    String(String),
}

impl std::fmt::Display for Value {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(value) => write!(formatter, "{value}"),
            Self::String(value) => write!(formatter, "{value:?}"),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ReferenceEvaluation {
    pub requirement: String,
    pub name: String,
    pub expression: String,
    pub passed: bool,
    pub detail: String,
}

pub(crate) fn evaluate(
    observations: &str,
    specification: &str,
) -> Result<Vec<ReferenceEvaluation>> {
    let mut environment = BTreeMap::new();
    let mut evaluations = Vec::new();
    evaluate_source(
        observations,
        "executor observations",
        &mut environment,
        &mut evaluations,
    )?;
    evaluate_source(
        specification,
        "specification",
        &mut environment,
        &mut evaluations,
    )?;

    match environment.get("contract-profile") {
        Some(Value::String(profile)) if profile == CONTRACT_PROFILE => {}
        Some(profile) => {
            return Err(SpecError::new(format!(
                "unsupported contract profile {profile}; expected {CONTRACT_PROFILE:?}",
            )));
        }
        None => {
            return Err(SpecError::new(
                "specification does not declare contract-profile",
            ));
        }
    }
    if evaluations.is_empty() {
        return Err(SpecError::new(
            "specification contains no integrity constraints",
        ));
    }
    Ok(evaluations)
}

fn evaluate_source(
    source: &str,
    label: &str,
    environment: &mut BTreeMap<String, Value>,
    evaluations: &mut Vec<ReferenceEvaluation>,
) -> Result<()> {
    let mut pending_requirement = None;
    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = raw_line.trim();
        if let Some(requirement) = trimmed.strip_prefix("-- @requirement ") {
            pending_requirement = Some(requirement.trim().to_string());
            continue;
        }
        let executable = strip_comment(raw_line).trim();
        let Some((raw_name, raw_expression)) = executable.split_once(":=") else {
            continue;
        };
        let name = raw_name.trim();
        let expression = raw_expression.trim();
        if name.is_empty() || expression.is_empty() {
            return Err(parse_error(label, line_number, "incomplete binding"));
        }

        if name.ends_with('!') {
            let requirement = pending_requirement.take().ok_or_else(|| {
                parse_error(
                    label,
                    line_number,
                    &format!("constraint `{name}` has no @requirement annotation"),
                )
            })?;
            let (passed, detail) = evaluate_constraint(expression, environment)
                .map_err(|message| parse_error(label, line_number, &message))?;
            environment.insert(name.to_string(), Value::Bool(passed));
            evaluations.push(ReferenceEvaluation {
                requirement,
                name: name.to_string(),
                expression: expression.to_string(),
                passed,
                detail,
            });
        } else {
            let value = resolve(expression, environment)
                .map_err(|message| parse_error(label, line_number, &message))?;
            if environment.insert(name.to_string(), value).is_some() {
                return Err(parse_error(
                    label,
                    line_number,
                    &format!("binding `{name}` is defined more than once"),
                ));
            }
        }
    }
    Ok(())
}

fn evaluate_constraint(
    expression: &str,
    environment: &BTreeMap<String, Value>,
) -> std::result::Result<(bool, String), String> {
    let Some(operator) = find_operator(expression, "===") else {
        return Err(format!(
            "expression `{expression}` is outside the mech-contract-1 prototype; expected strict equality",
        ));
    };
    let left = resolve(&expression[..operator], environment)?;
    let right = resolve(&expression[operator + 3..], environment)?;
    let passed = left == right;
    let detail = if passed {
        format!("{left} === {right}")
    } else {
        format!("observed {left}; expected {right}")
    };
    Ok((passed, detail))
}

fn resolve(
    expression: &str,
    environment: &BTreeMap<String, Value>,
) -> std::result::Result<Value, String> {
    let expression = expression.trim();
    if expression == "true" {
        return Ok(Value::Bool(true));
    }
    if expression == "false" {
        return Ok(Value::Bool(false));
    }
    if expression.starts_with('"') {
        let string = serde_json::from_str::<String>(expression)
            .map_err(|error| format!("invalid string literal: {error}"))?;
        return Ok(Value::String(string));
    }
    environment
        .get(expression)
        .cloned()
        .ok_or_else(|| format!("unknown binding `{expression}`"))
}

fn find_operator(expression: &str, needle: &str) -> Option<usize> {
    let bytes = expression.as_bytes();
    let needle = needle.as_bytes();
    let mut quoted = false;
    let mut escaped = false;
    let mut index = 0;
    while index + needle.len() <= bytes.len() {
        match bytes[index] {
            b'\\' if quoted => escaped = !escaped,
            b'"' if !escaped => quoted = !quoted,
            _ => escaped = false,
        }
        if !quoted && &bytes[index..index + needle.len()] == needle {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quoted = false;
    let mut escaped = false;
    let mut index = 0;
    while index + 1 < bytes.len() {
        match bytes[index] {
            b'\\' if quoted => escaped = !escaped,
            b'"' if !escaped => quoted = !quoted,
            _ => escaped = false,
        }
        if !quoted && bytes[index] == b'-' && bytes[index + 1] == b'-' {
            return &line[..index];
        }
        index += 1;
    }
    line
}

fn parse_error(label: &str, line: usize, message: &str) -> SpecError {
    SpecError::new(format!("{label}:{line}: {message}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_the_restricted_profile_independently() {
        let observations = "before := \"1\"\nafter := \"1\"\noutcome := \"abort\"\n";
        let specification = r#"
contract-profile := "mech-contract-1"
-- @requirement TURN-004
preserved! := before === after
-- @requirement TURN-003
aborted! := outcome === "abort"
"#;
        let results = evaluate(observations, specification).unwrap();
        assert!(results.iter().all(|result| result.passed));
    }

    #[test]
    fn reports_semantic_drift_as_a_failed_contract() {
        let observations = "before := \"1\"\nafter := \"2\"\n";
        let specification = r#"
contract-profile := "mech-contract-1"
-- @requirement TURN-004
preserved! := before === after
"#;
        let results = evaluate(observations, specification).unwrap();
        assert!(!results[0].passed);
        assert!(results[0].detail.contains("observed \"1\"; expected \"2\""));
    }
}
