use mech_core::{MResult, MechError, MechErrorKind, ValRef, Value, ValueStateJournal};

#[derive(Clone)]
pub(crate) struct BytecodeRegisterFile {
    cells: Vec<ValRef>,
}

impl BytecodeRegisterFile {
    pub fn new(register_count: usize) -> Self {
        Self {
            cells: (0..register_count)
                .map(|_| mech_core::Ref::new(Value::Empty))
                .collect(),
        }
    }

    pub fn load(&self, register: u32, value: Value) -> MResult<()> {
        let cell = self.cell(register)?;
        let mut current = cell.try_borrow_mut().map_err(|_| {
            register_error(
                register,
                self.cells.len(),
                "register cell is already borrowed",
            )
        })?;
        *current = value;
        Ok(())
    }

    pub fn cell(&self, register: u32) -> MResult<ValRef> {
        self.cells
            .get(register as usize)
            .cloned()
            .ok_or_else(|| register_error(register, self.cells.len(), "register is out of range"))
    }

    pub fn value(&self, register: u32) -> MResult<Value> {
        let cell = self.cell(register)?;
        cell.try_borrow().map(|value| value.clone()).map_err(|_| {
            register_error(
                register,
                self.cells.len(),
                "register cell is mutably borrowed",
            )
        })
    }

    pub fn function_argument(&self, register: u32) -> MResult<Value> {
        self.value(register)
    }

    pub fn external_input(&self, register: u32) -> MResult<Value> {
        Ok(Value::MutableReference(self.cell(register)?))
    }

    pub(crate) fn len(&self) -> usize {
        self.cells.len()
    }
}

#[derive(Clone)]
pub(crate) struct BytecodeRegisterFileCheckpoint {
    registers: BytecodeRegisterFile,
}

impl BytecodeRegisterFileCheckpoint {
    pub(crate) fn capture(
        registers: &BytecodeRegisterFile,
        journal: &mut ValueStateJournal,
    ) -> MResult<Self> {
        for cell in &registers.cells {
            journal.capture_val_ref(cell)?;
        }
        Ok(Self {
            registers: registers.clone(),
        })
    }

    pub(crate) fn restore(&self) -> BytecodeRegisterFile {
        self.registers.clone()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BytecodeRegisterAccessError {
    register: u32,
    register_count: usize,
    reason: &'static str,
}

impl MechErrorKind for BytecodeRegisterAccessError {
    fn name(&self) -> &str {
        "BytecodeRegisterAccess"
    }

    fn message(&self) -> String {
        format!(
            "cannot access bytecode register {} in a {}-register file: {}",
            self.register, self.register_count, self.reason,
        )
    }
}

fn register_error(register: u32, register_count: usize, reason: &'static str) -> MechError {
    MechError::new(
        BytecodeRegisterAccessError {
            register,
            register_count,
            reason,
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_preserve_the_register_cell_handle() {
        let registers = BytecodeRegisterFile::new(1);
        let cell = registers.cell(0).unwrap();

        registers
            .load(0, Value::Index(mech_core::Ref::new(1)))
            .unwrap();

        assert!(cell.same_handle(&registers.cell(0).unwrap()));
        assert_eq!(
            registers.value(0).unwrap(),
            Value::Index(mech_core::Ref::new(1))
        );
        assert!(matches!(
            registers.external_input(0).unwrap(),
            Value::MutableReference(reference) if reference.same_handle(&cell)
        ));
    }

    #[test]
    fn invalid_registers_are_structured_errors() {
        let registers = BytecodeRegisterFile::new(1);
        let error = registers.value(1).unwrap_err();
        assert_eq!(error.kind_name(), "BytecodeRegisterAccess");
    }
}
