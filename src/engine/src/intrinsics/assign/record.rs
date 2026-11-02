//! Public names retained after canonical record assignment replaced legacy kernels.

#[derive(Debug)]
pub struct RecordAssign<T>(core::marker::PhantomData<T>);

#[derive(Debug)]
pub struct AssignRecordField;

#[derive(Debug, Clone)]
pub struct UndefinedRecordFieldError {
    pub id: u64,
}

impl mech_core::MechErrorKind for UndefinedRecordFieldError {
    fn name(&self) -> &str {
        "UndefinedRecordField"
    }

    fn message(&self) -> String {
        format!("Field {:?} is not defined in this record.", self.id)
    }
}
