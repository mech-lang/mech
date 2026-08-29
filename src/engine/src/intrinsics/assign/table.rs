//! Public names retained after canonical table assignment replaced legacy kernels.

#[derive(Debug)]
pub struct AssignTableColumn;

#[derive(Debug)]
pub struct AddAssignTable;

#[derive(Debug, Clone)]
pub struct UndefinedTableColumnError {
    pub id: u64,
}

impl mech_core::MechErrorKind for UndefinedTableColumnError {
    fn name(&self) -> &str {
        "UndefinedTableColumn"
    }

    fn message(&self) -> String {
        format!("Column {:?} is not defined in this table.", self.id)
    }
}
