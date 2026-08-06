use crate::{BytecodeValidationError, MResult, MechError, Ref, Value, ValueKind};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, format, vec, vec::Vec};

/// Direct children whose producer cells define a composite's reactive topology.
/// This ordering is the canonical `CompositePack` argument ordering.
pub fn bytecode_composite_children(value: &Value) -> Option<Vec<Value>> {
    match value {
        #[cfg(feature = "tuple")]
        Value::Tuple(value) => Some(
            value
                .borrow()
                .elements
                .iter()
                .map(|value| (**value).clone())
                .collect(),
        ),
        #[cfg(feature = "record")]
        Value::Record(value) => Some(value.borrow().data.values().cloned().collect()),
        #[cfg(feature = "map")]
        Value::Map(value) => Some(
            value
                .borrow()
                .map
                .iter()
                .flat_map(|(key, value)| [key.clone(), value.clone()])
                .collect(),
        ),
        #[cfg(feature = "set")]
        Value::Set(value) => Some(value.borrow().set.iter().cloned().collect()),
        #[cfg(feature = "table")]
        Value::Table(value) => Some(
            value
                .borrow()
                .data
                .values()
                .flat_map(|(_, values)| values.as_vec())
                .collect(),
        ),
        #[cfg(feature = "enum")]
        Value::Enum(value) => Some(
            value
                .borrow()
                .variants
                .iter()
                .filter_map(|(_, payload)| payload.clone())
                .collect(),
        ),
        #[cfg(feature = "matrix")]
        Value::MatrixValue(value) => Some(value.as_vec()),
        Value::Typed(value, _) => Some(vec![(**value).clone()]),
        _ => None,
    }
}

fn wrong_arity(kind: &str, expected: usize, actual: usize) -> MechError {
    MechError::new(
        BytecodeValidationError {
            reason: format!("{kind} CompositePack expects {expected} children, found {actual}"),
        },
        None,
    )
    .with_compiler_loc()
}

fn duplicate_hashed_child(kind: &str) -> MechError {
    MechError::new(
        BytecodeValidationError {
            reason: format!(
                "{kind} CompositePack produced duplicate-equal hashed children after evaluation"
            ),
        },
        None,
    )
    .with_compiler_loc()
}

fn compiled_child_kind(value: &Value) -> ValueKind {
    let mut kind = value.kind();
    while let ValueKind::Reference(inner) = kind {
        kind = *inner;
    }
    kind
}

fn wrong_child_kind(index: usize, expected: &ValueKind, actual: &ValueKind) -> MechError {
    MechError::new(
        BytecodeValidationError {
            reason: format!(
                "CompositePack child {index} has kind {:?}, expected {:?} from the template schema",
                actual, expected,
            ),
        },
        None,
    )
    .with_compiler_loc()
}

/// Validates the exact child schema shared by bytecode reading, contract
/// planning, and runtime reconstruction.
pub fn validate_bytecode_composite_children(template: &Value, children: &[Value]) -> MResult<()> {
    let expected = bytecode_composite_children(template).ok_or_else(|| {
        MechError::new(
            BytecodeValidationError {
                reason: format!(
                    "CompositePack template kind {:?} is not structurally lowerable",
                    template.kind(),
                ),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    if children.len() != expected.len() {
        return Err(wrong_arity("Template", expected.len(), children.len()));
    }
    for (index, (expected, actual)) in expected.iter().zip(children).enumerate() {
        let expected_kind = compiled_child_kind(expected);
        let actual_kind = actual.kind();
        if actual_kind != expected_kind {
            return Err(wrong_child_kind(index, &expected_kind, &actual_kind));
        }
    }
    Ok(())
}

/// Rebuilds one composite layer from a constant template and live child values.
pub fn rebuild_bytecode_composite(template: &Value, children: Vec<Value>) -> MResult<Value> {
    validate_bytecode_composite_children(template, &children)?;
    match template {
        #[cfg(feature = "tuple")]
        Value::Tuple(value) => {
            let expected = value.borrow().elements.len();
            if children.len() != expected {
                return Err(wrong_arity("Tuple", expected, children.len()));
            }
            Ok(Value::Tuple(Ref::new(crate::MechTuple::from_vec(children))))
        }
        #[cfg(feature = "record")]
        Value::Record(value) => {
            let value = value.borrow();
            if children.len() != value.data.len() {
                return Err(wrong_arity("Record", value.data.len(), children.len()));
            }
            let fields = value
                .data
                .keys()
                .copied()
                .zip(children)
                .map(|(id, child)| {
                    (
                        id,
                        value.field_names.get(&id).cloned().unwrap_or_default(),
                        child,
                    )
                })
                .collect();
            Ok(Value::Record(Ref::new(crate::MechRecord::from_parts(
                value.cols,
                value.kinds.clone(),
                fields,
            ))))
        }
        #[cfg(feature = "map")]
        Value::Map(value) => {
            let value = value.borrow();
            let expected = value
                .map
                .len()
                .checked_mul(2)
                .ok_or_else(|| wrong_arity("Map", usize::MAX, children.len()))?;
            if children.len() != expected {
                return Err(wrong_arity("Map", expected, children.len()));
            }
            let mut children = children.into_iter();
            // Map keys must never retain producer-owned cells: mutating a key
            // after insertion invalidates the hash table's buckets. Values are
            // deliberately retained so a downstream access that captured the
            // original value cell continues to observe producer updates.
            let entries = (0..value.map.len())
                .map(|_| {
                    Ok((
                        children.next().unwrap().try_deep_snapshot()?,
                        children.next().unwrap(),
                    ))
                })
                .collect::<MResult<Vec<_>>>()?;
            let entry_count = entries.len();
            let rebuilt = crate::MechMap::from_typed_vec(
                value.key_kind.clone(),
                value.value_kind.clone(),
                value.num_elements,
                entries,
            );
            if rebuilt.map.len() != entry_count {
                return Err(duplicate_hashed_child("Map"));
            }
            Ok(Value::Map(Ref::new(rebuilt)))
        }
        #[cfg(feature = "set")]
        Value::Set(value) => {
            let value = value.borrow();
            if children.len() != value.set.len() {
                return Err(wrong_arity("Set", value.set.len(), children.len()));
            }
            // Set elements are their own hashed identity and therefore need
            // the same producer-detachment rule as map keys.
            let expected = children.len();
            let children = children
                .iter()
                .map(Value::try_deep_snapshot)
                .collect::<MResult<Vec<_>>>()?;
            let mut rebuilt = crate::MechSet::from_vec(children);
            if rebuilt.set.len() != expected {
                return Err(duplicate_hashed_child("Set"));
            }
            rebuilt.kind = value.kind.clone();
            rebuilt.max_elements = value.max_elements;
            rebuilt.num_elements = value.num_elements;
            Ok(Value::Set(Ref::new(rebuilt)))
        }
        #[cfg(feature = "table")]
        Value::Table(value) => {
            let value = value.borrow();
            let expected = value
                .data
                .values()
                .try_fold(0usize, |total, (_, matrix)| {
                    total.checked_add(matrix.rows().saturating_mul(matrix.cols()))
                })
                .ok_or_else(|| wrong_arity("Table", usize::MAX, children.len()))?;
            if children.len() != expected {
                return Err(wrong_arity("Table", expected, children.len()));
            }
            let mut children = children.into_iter();
            let columns = value
                .data
                .iter()
                .map(|(id, (kind, matrix))| {
                    let len = matrix.rows().saturating_mul(matrix.cols());
                    let elements = children.by_ref().take(len).collect::<Vec<_>>();
                    (
                        *id,
                        kind.clone(),
                        matrix.rebuild_with_same_storage(elements, matrix.rows(), matrix.cols()),
                    )
                })
                .collect();
            Ok(Value::Table(Ref::new(crate::MechTable::from_parts(
                value.rows,
                value.cols,
                columns,
                value
                    .col_names
                    .iter()
                    .map(|(id, name)| (*id, name.clone()))
                    .collect(),
            ))))
        }
        #[cfg(feature = "enum")]
        Value::Enum(value) => {
            let value = value.borrow();
            let expected = value
                .variants
                .iter()
                .filter(|(_, payload)| payload.is_some())
                .count();
            if children.len() != expected {
                return Err(wrong_arity("Enum", expected, children.len()));
            }
            let mut children = children.into_iter();
            Ok(Value::Enum(Ref::new(crate::MechEnum {
                id: value.id,
                variants: value
                    .variants
                    .iter()
                    .map(|(id, payload)| (*id, payload.as_ref().map(|_| children.next().unwrap())))
                    .collect(),
                names: Ref::new(value.names.borrow().clone()),
            })))
        }
        #[cfg(feature = "matrix")]
        Value::MatrixValue(value) => {
            let expected = value.rows().saturating_mul(value.cols());
            if children.len() != expected {
                return Err(wrong_arity("MatrixValue", expected, children.len()));
            }
            Ok(Value::MatrixValue(value.rebuild_with_same_storage(
                children,
                value.rows(),
                value.cols(),
            )))
        }
        Value::Typed(_, kind) => {
            if children.len() != 1 {
                return Err(wrong_arity("Typed", 1, children.len()));
            }
            Ok(Value::Typed(
                Box::new(children.into_iter().next().unwrap()),
                kind.clone(),
            ))
        }
        _ => Err(MechError::new(
            BytecodeValidationError {
                reason: format!(
                    "CompositePack template kind {:?} is not structurally lowerable",
                    template.kind(),
                ),
            },
            None,
        )
        .with_compiler_loc()),
    }
}
