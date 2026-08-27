#[cfg(any(
    feature = "tuple",
    feature = "record",
    feature = "map",
    feature = "set",
    feature = "table",
    feature = "enum"
))]
use crate::Ref;
#[cfg(feature = "matrix")]
use crate::ToValue;
use crate::{AsValueKind, BytecodeValidationError, LegacyValue, MResult, MechError, ValueKind};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, format, vec, vec::Vec};

/// Direct children whose producer cells define a composite's reactive topology.
/// This ordering is the canonical `CompositePack` argument ordering.
pub fn bytecode_composite_children(value: &LegacyValue) -> Option<Vec<LegacyValue>> {
    match value {
        #[cfg(feature = "tuple")]
        LegacyValue::Tuple(value) => Some(
            value
                .borrow()
                .elements
                .iter()
                .map(|value| (**value).clone())
                .collect(),
        ),
        #[cfg(feature = "record")]
        LegacyValue::Record(value) => Some(value.borrow().data.values().cloned().collect()),
        #[cfg(feature = "map")]
        LegacyValue::Map(value) => Some(
            value
                .borrow()
                .map
                .iter()
                .flat_map(|(key, value)| [key.clone(), value.clone()])
                .collect(),
        ),
        #[cfg(feature = "set")]
        LegacyValue::Set(value) => Some(value.borrow().set.iter().cloned().collect()),
        #[cfg(feature = "table")]
        LegacyValue::Table(value) => Some(
            value
                .borrow()
                .data
                .values()
                .flat_map(|(_, values)| values.as_vec())
                .collect(),
        ),
        #[cfg(feature = "enum")]
        LegacyValue::Enum(value) => Some(
            value
                .borrow()
                .variants
                .iter()
                .filter_map(|(_, payload)| payload.clone())
                .collect(),
        ),
        #[cfg(feature = "matrix")]
        LegacyValue::MatrixValue(value) => Some(value.as_vec()),
        LegacyValue::Typed(value, _) => Some(vec![(**value).clone()]),
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

#[cfg(any(feature = "map", feature = "set"))]
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

impl ValueKind {
    /// Returns the element kind and declared dimensions of an exact matrix
    /// kind without erasing its shape as the general collection helper does.
    pub fn matrix_parts(&self) -> Option<(&ValueKind, &[usize])> {
        match self {
            ValueKind::Matrix(element, dimensions) => {
                Some((element.as_ref(), dimensions.as_slice()))
            }
            _ => None,
        }
    }

    pub fn is_any(&self) -> bool {
        self == &<LegacyValue as AsValueKind>::as_value_kind()
    }

    pub fn option_inner(&self) -> Option<&ValueKind> {
        match self {
            ValueKind::Option(inner) => Some(inner.as_ref()),
            _ => None,
        }
    }
}

fn compiled_kind(kind: ValueKind) -> ValueKind {
    if let Some((inner, shape)) = kind
        .matrix_parts()
        .map(|(inner, shape)| (inner.clone(), shape.to_vec()))
    {
        return ValueKind::Matrix(Box::new(compiled_kind(inner)), shape);
    }
    if let Some(inner) = kind.option_inner().cloned() {
        return ValueKind::Option(Box::new(compiled_kind(inner)));
    }
    match kind {
        ValueKind::Reference(inner) => compiled_kind(*inner),
        ValueKind::Record(fields) => ValueKind::Record(
            fields
                .into_iter()
                .map(|(name, kind)| (name, compiled_kind(kind)))
                .collect(),
        ),
        ValueKind::Map(key, value) => ValueKind::Map(
            Box::new(compiled_kind(*key)),
            Box::new(compiled_kind(*value)),
        ),
        ValueKind::Table(columns, primary_key) => ValueKind::Table(
            columns
                .into_iter()
                .map(|(name, kind)| (name, compiled_kind(kind)))
                .collect(),
            primary_key,
        ),
        ValueKind::Tuple(elements) => {
            ValueKind::Tuple(elements.into_iter().map(compiled_kind).collect())
        }
        ValueKind::Set(element, max_len) => {
            ValueKind::Set(Box::new(compiled_kind(*element)), max_len)
        }
        ValueKind::Kind(inner) => ValueKind::Kind(Box::new(compiled_kind(*inner))),
        kind => kind,
    }
}

fn compiled_child_kind(value: &LegacyValue) -> ValueKind {
    compiled_kind(value.kind())
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

#[cfg(feature = "matrix")]
fn matrix_template_schema(template: &LegacyValue) -> MResult<Option<(ValueKind, usize, usize)>> {
    let Some(kind) = template.legacy_kind_literal() else {
        return Ok(None);
    };
    let Some((element_kind, dimensions)) = kind.matrix_parts() else {
        return Ok(None);
    };
    let [rows, columns] = dimensions else {
        return Err(MechError::new(
            BytecodeValidationError {
                reason: format!(
                    "Matrix CompositePack template requires exactly two dimensions, found {}",
                    dimensions.len(),
                ),
            },
            None,
        )
        .with_compiler_loc());
    };
    Ok(Some((element_kind.clone(), *rows, *columns)))
}

#[cfg(feature = "matrix")]
fn validate_matrix_child(
    index: usize,
    element_kind: &ValueKind,
    child: &LegacyValue,
) -> MResult<()> {
    if let Some(reference) = child.legacy_mutable_reference() {
        return validate_matrix_child(index, element_kind, &reference.borrow());
    }

    let expected = compiled_kind(element_kind.clone());
    let actual = compiled_kind(child.kind());
    let compatible = if child.is_legacy_index_all() || child.legacy_kind_literal().is_some() {
        false
    } else if child.is_legacy_empty() {
        expected.is_any() || expected.option_inner().is_some()
    } else if let Some(empty_kind) = child.legacy_empty_kind() {
        empty_kind.option_inner().is_some() && compiled_kind(empty_kind.clone()) == expected
    } else if expected.is_any() {
        true
    } else if let Some(inner) = expected.option_inner() {
        actual == expected || &actual == inner
    } else {
        actual == expected
    };
    if compatible {
        Ok(())
    } else {
        Err(wrong_child_kind(index, element_kind, &child.kind()))
    }
}

/// Validates the exact child schema shared by bytecode reading, contract
/// planning, and runtime reconstruction.
pub fn validate_bytecode_composite_children(
    template: &LegacyValue,
    children: &[LegacyValue],
) -> MResult<()> {
    #[cfg(feature = "matrix")]
    if let Some((element_kind, rows, columns)) = matrix_template_schema(template)? {
        let expected = rows
            .checked_mul(columns)
            .ok_or_else(|| wrong_arity("Matrix", usize::MAX, children.len()))?;
        if children.len() != expected {
            return Err(wrong_arity("Matrix", expected, children.len()));
        }
        for (index, child) in children.iter().enumerate() {
            validate_matrix_child(index, &element_kind, child)?;
        }
        return Ok(());
    }

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
        if !matches!(expected_kind, ValueKind::Any) && actual_kind != expected_kind {
            return Err(wrong_child_kind(index, &expected_kind, &actual_kind));
        }
    }
    Ok(())
}

/// Rebuilds one composite layer from a constant template and live child values.
pub fn rebuild_bytecode_composite(
    template: &LegacyValue,
    children: Vec<LegacyValue>,
) -> MResult<LegacyValue> {
    validate_bytecode_composite_children(template, &children)?;
    #[cfg(feature = "matrix")]
    if let Some((_, rows, columns)) = matrix_template_schema(template)? {
        let mut column_major = Vec::with_capacity(children.len());
        for column in 0..columns {
            for row in 0..rows {
                column_major.push(children[row * columns + column].clone());
            }
        }
        return Ok(crate::matrix::Matrix::from_vec(column_major, rows, columns).to_value());
    }
    match template {
        #[cfg(feature = "tuple")]
        LegacyValue::Tuple(value) => {
            let expected = value.borrow().elements.len();
            if children.len() != expected {
                return Err(wrong_arity("Tuple", expected, children.len()));
            }
            Ok(LegacyValue::Tuple(Ref::new(crate::MechTuple::from_vec(
                children,
            ))))
        }
        #[cfg(feature = "record")]
        LegacyValue::Record(value) => {
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
            Ok(LegacyValue::Record(Ref::new(
                crate::MechRecord::from_parts(value.cols, value.kinds.clone(), fields),
            )))
        }
        #[cfg(feature = "map")]
        LegacyValue::Map(value) => {
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
            Ok(LegacyValue::Map(Ref::new(rebuilt)))
        }
        #[cfg(feature = "set")]
        LegacyValue::Set(value) => {
            let value = value.borrow();
            if children.len() != value.set.len() {
                return Err(wrong_arity("Set", value.set.len(), children.len()));
            }
            // Set elements are their own hashed identity and therefore need
            // the same producer-detachment rule as map keys.
            let expected = children.len();
            let children = children
                .iter()
                .map(LegacyValue::try_deep_snapshot)
                .collect::<MResult<Vec<_>>>()?;
            let mut rebuilt = crate::MechSet::from_vec(children);
            if rebuilt.set.len() != expected {
                return Err(duplicate_hashed_child("Set"));
            }
            rebuilt.kind = value.kind.clone();
            rebuilt.max_elements = value.max_elements;
            rebuilt.num_elements = value.num_elements;
            Ok(LegacyValue::Set(Ref::new(rebuilt)))
        }
        #[cfg(feature = "table")]
        LegacyValue::Table(value) => {
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
            Ok(LegacyValue::Table(Ref::new(crate::MechTable::from_parts(
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
        LegacyValue::Enum(value) => {
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
            Ok(LegacyValue::Enum(Ref::new(crate::MechEnum {
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
        LegacyValue::MatrixValue(value) => {
            let expected = value.rows().saturating_mul(value.cols());
            if children.len() != expected {
                return Err(wrong_arity("MatrixValue", expected, children.len()));
            }
            Ok(LegacyValue::MatrixValue(value.rebuild_with_same_storage(
                children,
                value.rows(),
                value.cols(),
            )))
        }
        LegacyValue::Typed(_, kind) => {
            if children.len() != 1 {
                return Err(wrong_arity("Typed", 1, children.len()));
            }
            Ok(LegacyValue::Typed(
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

#[cfg(all(test, feature = "matrix", feature = "f64", feature = "string"))]
mod tests {
    use super::*;
    use crate::Ref;

    fn f64_value(value: f64) -> LegacyValue {
        LegacyValue::F64(Ref::new(value))
    }

    fn matrix_template(element: ValueKind, rows: usize, columns: usize) -> LegacyValue {
        LegacyValue::Kind(ValueKind::Matrix(Box::new(element), vec![rows, columns]))
    }

    #[test]
    fn matrix_kind_template_rebuilds_logical_row_major_order() {
        let rebuilt = rebuild_bytecode_composite(
            &matrix_template(ValueKind::F64, 2, 3),
            (1..=6).map(|value| f64_value(value as f64)).collect(),
        )
        .unwrap();
        let LegacyValue::MatrixValue(matrix) = rebuilt else {
            panic!("matrix-kind template must rebuild a generic matrix");
        };
        let logical = (1..=2)
            .flat_map(|row| (1..=3).map(move |column| (row, column)))
            .map(|(row, column)| {
                let LegacyValue::F64(value) = matrix.index2d(row, column) else {
                    panic!("rebuilt matrix element must be f64");
                };
                *value.borrow()
            })
            .collect::<Vec<_>>();
        assert_eq!(logical, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn matrix_kind_template_rejects_wrong_count_kind_and_controls() {
        assert!(
            rebuild_bytecode_composite(
                &matrix_template(ValueKind::F64, 1, 2),
                vec![f64_value(1.0)],
            )
            .is_err()
        );
        assert!(
            rebuild_bytecode_composite(
                &matrix_template(ValueKind::F64, 1, 1),
                vec![LegacyValue::String(Ref::new("wrong".into()))],
            )
            .is_err()
        );
        assert!(
            rebuild_bytecode_composite(
                &matrix_template(ValueKind::F64, 1, 1),
                vec![LegacyValue::Empty],
            )
            .is_err()
        );
        assert!(
            rebuild_bytecode_composite(
                &matrix_template(ValueKind::Any, 1, 1),
                vec![LegacyValue::IndexAll],
            )
            .is_err()
        );
        assert!(
            rebuild_bytecode_composite(
                &matrix_template(ValueKind::Any, 1, 1),
                vec![LegacyValue::Kind(ValueKind::F64)],
            )
            .is_err()
        );
    }

    #[test]
    fn matrix_any_and_option_schemas_accept_their_legacy_compatibility_values() {
        let heterogeneous = vec![f64_value(1.0), LegacyValue::String(Ref::new("two".into()))];
        assert!(
            rebuild_bytecode_composite(&matrix_template(ValueKind::Any, 1, 2), heterogeneous,)
                .is_ok()
        );

        let option = ValueKind::Option(Box::new(ValueKind::F64));
        let values = vec![
            f64_value(1.0),
            LegacyValue::Typed(Box::new(f64_value(2.0)), option.clone()),
            LegacyValue::Empty,
            LegacyValue::EmptyKind(option.clone()),
        ];
        let rebuilt = rebuild_bytecode_composite(&matrix_template(option, 2, 2), values).unwrap();
        let LegacyValue::MatrixValue(matrix) = rebuilt else {
            panic!("2x2 option matrix should retain generic matrix storage");
        };
        assert_eq!(matrix.as_vec().len(), 4);
    }

    #[test]
    fn matrix_schema_checks_the_current_mutable_reference_payload() {
        let valid = LegacyValue::MutableReference(Ref::new(f64_value(1.0)));
        assert!(
            validate_bytecode_composite_children(&matrix_template(ValueKind::F64, 1, 1), &[valid],)
                .is_ok()
        );

        let invalid =
            LegacyValue::MutableReference(Ref::new(LegacyValue::String(Ref::new("wrong".into()))));
        assert!(
            validate_bytecode_composite_children(
                &matrix_template(ValueKind::F64, 1, 1),
                &[invalid],
            )
            .is_err()
        );
    }
}
