use crate::MechErrorKind;
#[cfg(feature = "record")]
use crate::ValueKind;

#[derive(Debug, Clone)]
pub struct AddressedAssignmentUnsupported;

impl MechErrorKind for AddressedAssignmentUnsupported {
    fn name(&self) -> &str {
        "AddressedAssignmentUnsupported"
    }
    fn message(&self) -> String {
        "addressed assignment is not supported yet".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct UnableToConvertAtomToEnumVariantError {
    pub atom_name: String,
    pub target_enum_variant_name: String,
}

impl MechErrorKind for UnableToConvertAtomToEnumVariantError {
    fn name(&self) -> &str {
        "UnableToConvertAtomToEnumVariant"
    }
    fn message(&self) -> String {
        format!(
            "Unable to convert atom variant `{} to enum <{}>",
            self.atom_name, self.target_enum_variant_name
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnableToConvertAtomError {
    pub atom_id: u64,
}

impl MechErrorKind for UnableToConvertAtomError {
    fn name(&self) -> &str {
        "UnableToConvertAtom"
    }
    fn message(&self) -> String {
        format!("Unable to atom  {}", self.atom_id)
    }
}

#[derive(Debug, Clone)]
pub struct VariableAlreadyDefinedError {
    pub id: u64,
}

impl MechErrorKind for VariableAlreadyDefinedError {
    fn name(&self) -> &str {
        "VariableAlreadyDefined"
    }
    fn message(&self) -> String {
        format!("Variable already defined: {}", self.id)
    }
}

#[derive(Debug, Clone)]
pub struct UndefinedVariableError {
    pub id: u64,
    pub name: String,
}

impl MechErrorKind for UndefinedVariableError {
    fn name(&self) -> &str {
        "UndefinedVariable"
    }

    fn message(&self) -> String {
        format!("Undefined variable `{}` (id: {})", self.name, self.id)
    }
}

#[derive(Debug, Clone)]
pub struct NotMutableError {
    pub id: u64,
}

impl MechErrorKind for NotMutableError {
    fn name(&self) -> &str {
        "NotMutable"
    }
    fn message(&self) -> String {
        format!("Variable is not mutable: {}", self.id)
    }
}

#[cfg(feature = "record")]
#[derive(Debug, Clone)]
pub struct UnableToConvertRecordError {
    pub source_record_kind: ValueKind,
    pub target_record_kind: ValueKind,
}

#[cfg(feature = "record")]
impl MechErrorKind for UnableToConvertRecordError {
    fn name(&self) -> &str {
        "UnableToConvertRecord"
    }
    fn message(&self) -> String {
        format!(
            "Unable to convert record of kind `{:?}` to record of kind `{:?}`",
            self.source_record_kind, self.target_record_kind
        )
    }
}
