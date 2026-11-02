use crate::{BytecodeValidationError, MResult, MechError, Register};

/// One source element in a generic matrix-literal construction.
///
/// This metadata is compiler-local and register based. `Empty` preserves the
/// source pseudo-value distinction until canonical artifact lowering resolves
/// it against the declared element schema.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompiledMatrixLiteralElement {
    Empty { register: Register },
    Value { register: Register },
}

impl CompiledMatrixLiteralElement {
    pub const fn register(self) -> Register {
        match self {
            Self::Empty { register } | Self::Value { register } => register,
        }
    }
}

/// Deterministic compiler sidecar for one generic matrix construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledMatrixLiteral {
    pub output: Register,
    pub rows: u32,
    pub columns: u32,
    pub elements: Box<[CompiledMatrixLiteralElement]>,
}

impl CompiledMatrixLiteral {
    pub fn new(
        output: Register,
        rows: u32,
        columns: u32,
        elements: Box<[CompiledMatrixLiteralElement]>,
    ) -> MResult<Self> {
        let expected = rows.checked_mul(columns).ok_or_else(|| {
            invalid::<()>(format!(
                "matrix literal dimensions {rows}x{columns} overflow element cardinality",
            ))
            .unwrap_err()
        })?;
        if usize::try_from(expected).ok() != Some(elements.len()) {
            return invalid(format!(
                "matrix literal dimensions {rows}x{columns} require {expected} elements, found {}",
                elements.len(),
            ));
        }
        if elements.iter().any(|element| element.register() == output) {
            return invalid(format!(
                "matrix literal output register {output} cannot also be an element register",
            ));
        }
        Ok(Self {
            output,
            rows,
            columns,
            elements,
        })
    }
}

fn invalid<T>(reason: impl Into<String>) -> MResult<T> {
    Err(MechError::new(
        BytecodeValidationError {
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_matrix_literal_accepts_repeated_inputs() {
        let literal = CompiledMatrixLiteral::new(
            2,
            1,
            2,
            vec![
                CompiledMatrixLiteralElement::Value { register: 1 },
                CompiledMatrixLiteralElement::Value { register: 1 },
            ]
            .into_boxed_slice(),
        )
        .unwrap();

        assert_eq!(literal.elements[0].register(), 1);
        assert_eq!(literal.elements[1].register(), 1);
    }

    #[test]
    fn checked_matrix_literal_rejects_invalid_cardinality_and_output_cycles() {
        let cardinality = CompiledMatrixLiteral::new(
            2,
            2,
            2,
            vec![CompiledMatrixLiteralElement::Value { register: 1 }].into_boxed_slice(),
        )
        .unwrap_err();
        assert!(cardinality.kind_message().contains("require 4 elements"));

        let cycle = CompiledMatrixLiteral::new(
            2,
            1,
            1,
            vec![CompiledMatrixLiteralElement::Value { register: 2 }].into_boxed_slice(),
        )
        .unwrap_err();
        assert!(cycle.kind_message().contains("cannot also be an element"));
    }

    #[test]
    fn checked_matrix_literal_rejects_dimension_overflow() {
        let error = CompiledMatrixLiteral::new(u32::MAX, u32::MAX, 2, Box::new([])).unwrap_err();
        assert!(error.kind_message().contains("overflow"));
    }
}
