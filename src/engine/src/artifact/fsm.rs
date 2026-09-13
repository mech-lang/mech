//! Typed finite-state-machine invocation data.
//!
//! FSM values are retained as structure plus references to the enclosing
//! node's ordinary input bindings. Source text remains diagnostic metadata and
//! is never consulted to reconstruct transition behavior.

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FsmValue {
    Input(u16),
    Tuple(Box<[FsmValue]>),
    Array(Box<[FsmValue]>),
    AtomStruct {
        name: String,
        items: Box<[FsmValue]>,
    },
    TupleStruct {
        name: String,
        items: Box<[FsmValue]>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FsmStageKind {
    State,
    Async,
    Output,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FsmStage {
    pub kind: FsmStageKind,
    pub value: FsmValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FsmArgument {
    pub name: Option<String>,
    pub input: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FsmDeclaration {
    pub machine: String,
    pub arguments: Box<[FsmArgument]>,
    pub stages: Box<[FsmStage]>,
}

pub(super) const MAX_FSM_VALUE_DEPTH: usize = 32;
pub(super) const MAX_FSM_STAGES: usize = 4_096;

// Mirrors the canonical syntax identifier's alphabetic, numeric, symbol, and
// forbidden-emoji scalar classes without depending on the source parser.
fn is_forbidden_identifier_emoji(character: char) -> bool {
    matches!(
        character,
        '\u{00a0}'
            | '\u{2009}'
            | '\u{27e8}'
            | '\u{27e9}'
            | '\u{2e22}'
            | '\u{2e25}'
            | '╭'
            | '╮'
            | '╰'
            | '╯'
            | '┏'
            | '┓'
            | '┗'
            | '┌'
            | '┐'
            | '└'
            | '┼'
            | '─'
            | '├'
            | '┤'
            | '┬'
            | '┴'
            | '│'
            | '┃'
    )
}

fn is_identifier_emoji(character: char) -> bool {
    !character.is_alphanumeric()
        && !character.is_ascii()
        && !is_forbidden_identifier_emoji(character)
}

fn is_canonical_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if !first.is_alphabetic() && !is_identifier_emoji(first) {
        return false;
    }
    characters.all(|character| {
        character.is_alphanumeric()
            || is_identifier_emoji(character)
            || matches!(
                character,
                '&' | '$' | '%' | '/' | '#' | '\\' | '~' | '+' | '-' | '*' | '^'
            )
    })
}

pub(super) fn value_count(value: &FsmValue) -> Option<usize> {
    let mut pending = vec![(value, 1usize)];
    let mut count = 0usize;
    while let Some((value, depth)) = pending.pop() {
        count = count.checked_add(1)?;
        if depth > MAX_FSM_VALUE_DEPTH || count > super::MAX_CONTROL_OPERANDS {
            return None;
        }
        let children = match value {
            FsmValue::Input(_) => continue,
            FsmValue::Tuple(items) | FsmValue::Array(items) => items,
            FsmValue::AtomStruct { items, .. } | FsmValue::TupleStruct { items, .. } => items,
        };
        if pending.len().checked_add(children.len())? > super::MAX_CONTROL_OPERANDS {
            return None;
        }
        pending.extend(children.iter().map(|item| (item, depth + 1)));
    }
    Some(count)
}

pub(super) fn validate_fsm(
    node: mech_core::NodeId,
    declaration: &FsmDeclaration,
    inputs: &[mech_core::SchemaId],
) -> Result<(), super::ArtifactBuildError> {
    let invalid = |reason| super::ArtifactBuildError::InvalidControl { node, reason };
    if !is_canonical_identifier(&declaration.machine) {
        return Err(invalid("FSM machine name is not a canonical identifier"));
    }
    if declaration.stages.len() > MAX_FSM_STAGES {
        return Err(invalid("FSM stage admission limit"));
    }
    let mut used = std::collections::BTreeSet::new();
    for argument in &declaration.arguments {
        if argument
            .name
            .as_deref()
            .is_some_and(|name| !is_canonical_identifier(name))
        {
            return Err(invalid("FSM argument name is not a canonical identifier"));
        }
        if inputs.get(argument.input as usize).is_none() || !used.insert(argument.input) {
            return Err(invalid("invalid or duplicate FSM argument input"));
        }
    }
    for stage in &declaration.stages {
        value_count(&stage.value).ok_or_else(|| invalid("FSM value admission limit"))?;
        let mut pending = vec![&stage.value];
        while let Some(value) = pending.pop() {
            match value {
                FsmValue::Input(input) => {
                    if inputs.get(*input as usize).is_none() || !used.insert(*input) {
                        return Err(invalid("invalid or duplicate FSM value input"));
                    }
                }
                FsmValue::Tuple(items) | FsmValue::Array(items) => pending.extend(items),
                FsmValue::AtomStruct { name, items } | FsmValue::TupleStruct { name, items } => {
                    if !is_canonical_identifier(name) {
                        return Err(invalid(
                            "FSM structured value name is not a canonical identifier",
                        ));
                    }
                    pending.extend(items);
                }
            }
        }
    }
    if used.len() != inputs.len() {
        return Err(invalid("every FSM input must have one typed role"));
    }
    Ok(())
}
