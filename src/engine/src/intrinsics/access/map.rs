//! Public names retained for canonical map-access diagnostics and artifacts.

#[derive(Debug)]
pub struct MapAccessField;

#[derive(Debug)]
pub struct MapAccess;

#[derive(Debug, Clone)]
pub struct UndefinedMapKeyError {
    pub key: String,
}

impl mech_core::MechErrorKind for UndefinedMapKeyError {
    fn name(&self) -> &str {
        "UndefinedMapKey"
    }

    fn message(&self) -> String {
        format!("Key `{}` was not found in the canonical map.", self.key)
    }
}
