use super::execute_access_function;
use crate::{
    InterpreterExecution, MResult, SchemaBody, SpecializationInput, Subscript, ValueCell, real,
};
use mech_core::snapshot::ValueDataDraft;

pub(super) fn access(
    subscript: &Subscript,
    value: &ValueCell,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    match subscript {
        #[cfg(feature = "table")]
        Subscript::Dot(identifier) => {
            let key =
                ValueCell::from_schema_data(SchemaBody::Id, ValueDataDraft::Id(identifier.hash()))?;
            execute_access_function(
                p,
                "access/column",
                vec![
                    SpecializationInput::Cell(value.clone()),
                    SpecializationInput::Cell(key),
                ],
            )
        }
        Subscript::DotInt(number) => {
            let index = real(number, p)?;
            let index =
                crate::intrinsics::access::matrix::canonical_reactive_scalar_index(index, p)?;
            execute_access_function(
                p,
                "access/scalar",
                vec![
                    SpecializationInput::Cell(value.clone()),
                    SpecializationInput::Cell(index),
                ],
            )
        }
        #[cfg(feature = "swizzle")]
        Subscript::Swizzle(identifiers) => {
            let mut inputs = Vec::with_capacity(identifiers.len() + 1);
            inputs.push(SpecializationInput::Cell(value.clone()));
            for identifier in identifiers {
                inputs.push(SpecializationInput::Cell(ValueCell::from_schema_data(
                    SchemaBody::Id,
                    ValueDataDraft::Id(identifier.hash()),
                )?));
            }
            execute_access_function(p, "access/swizzle", inputs)
        }
        _ => unreachable!(),
    }
}
