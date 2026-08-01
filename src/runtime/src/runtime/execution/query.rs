use crate::RuntimeValueSnapshot;
use crate::runtime::{MechRuntime, RuntimeInvalidOperationError, RuntimeProgramBusy};
use mech_core::{MResult, MechError, MechSourceCode, Program, SectionElement, Value};

impl MechRuntime {
    #[cfg(feature = "invariant_define")]
    pub fn integrity_constraint_report(&self) -> MResult<mech_engine::IntegrityConstraintReport> {
        self.program.integrity_constraint_report()
    }

    pub fn out_string(&self) -> String {
        self.program.out_string()
    }

    pub fn has_interpreter(&self, interpreter_id: u64) -> bool {
        self.program.has_interpreter(interpreter_id)
    }

    pub fn root_interpreter_id(&self) -> u64 {
        self.program.interpreter().id
    }

    pub fn root_plan_len(&self) -> usize {
        self.program.interpreter().plan_len()
    }

    /// Resolves a named fenced-document interpreter to its stable runtime ID.
    ///
    /// The source metadata is retained by the root interpreter, while the
    /// hierarchy confirms that the corresponding interpreter still exists.
    /// Callers receive only an ID; no live interpreter or borrow escapes this
    /// query boundary.
    pub fn interpreter_id_by_name(&self, name: &str) -> MResult<Option<u64>> {
        let mut candidate = None;
        for source in &self.program.interpreter().code {
            collect_interpreter_id_from_source(source, name, &mut candidate);
        }
        Ok(candidate.filter(|id| self.program.has_interpreter(*id)))
    }

    pub fn output_value_for_interpreter(
        &self,
        interpreter_id: u64,
        output_id: u64,
    ) -> MResult<Option<RuntimeValueSnapshot>> {
        self.program
            .output_value_for_interpreter(interpreter_id, output_id)
            .as_ref()
            .map(RuntimeValueSnapshot::try_capture)
            .transpose()
    }

    pub fn symbol_name_for_interpreter_output(
        &self,
        interpreter_id: u64,
        output_id: u64,
    ) -> Option<String> {
        self.program
            .symbol_name_for_interpreter_output(interpreter_id, output_id)
    }

    pub fn symbol_values_for_interpreter(
        &self,
        interpreter_id: u64,
        names: &[String],
    ) -> MResult<Option<Vec<(String, RuntimeValueSnapshot)>>> {
        self.program
            .symbol_values_for_interpreter(interpreter_id, names)
            .map(|values| {
                values
                    .into_iter()
                    .map(|(name, value)| Ok((name, RuntimeValueSnapshot::try_capture(&value)?)))
                    .collect::<MResult<Vec<_>>>()
            })
            .transpose()
    }

    pub fn root_symbol_value(&self, name: &str) -> MResult<RuntimeValueSnapshot> {
        self.program
            .root_symbol_value(name)
            .and_then(|value| RuntimeValueSnapshot::try_capture(&value))
    }

    pub fn root_symbol_values(
        &self,
        names: &[&str],
    ) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        self.program.root_symbol_values(names).and_then(|values| {
            values
                .into_iter()
                .map(|(name, value)| Ok((name, RuntimeValueSnapshot::try_capture(&value)?)))
                .collect::<MResult<Vec<_>>>()
        })
    }

    pub fn root_symbol_values_all(&self) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        self.program
            .root_symbol_values_all()
            .into_iter()
            .map(|(name, value)| Ok((name, RuntimeValueSnapshot::try_capture(&value)?)))
            .collect::<MResult<Vec<_>>>()
    }

    pub fn bind_ans_for_interpreter(&mut self, interpreter_id: u64, value: &Value) -> MResult<()> {
        if let Some(owner) = self.program_transaction_owner {
            return Err(MechError::new(
                RuntimeProgramBusy {
                    operation: "bind_ans_for_interpreter",
                    owner,
                    requester: None,
                },
                None,
            ));
        }

        if self.program.bind_ans_for_interpreter(interpreter_id, value) {
            return Ok(());
        }

        Err(MechError::new(
            RuntimeInvalidOperationError {
                operation: "bind_ans_for_interpreter",
                reason: format!("interpreter id {} not found", interpreter_id),
            },
            None,
        ))
    }
}

fn collect_interpreter_id_from_source(
    source: &MechSourceCode,
    name: &str,
    candidate: &mut Option<u64>,
) {
    match source {
        MechSourceCode::Tree(tree) => {
            collect_interpreter_id_from_program(tree, name, candidate);
        }
        MechSourceCode::Program(sources) => {
            for source in sources {
                collect_interpreter_id_from_source(source, name, candidate);
            }
        }
        _ => {}
    }
}

fn collect_interpreter_id_from_program(program: &Program, name: &str, candidate: &mut Option<u64>) {
    for section in &program.body.sections {
        for element in &section.elements {
            collect_interpreter_id_from_element(element, name, candidate);
        }
    }
}

fn collect_interpreter_id_from_element(
    element: &SectionElement,
    name: &str,
    candidate: &mut Option<u64>,
) {
    match element {
        SectionElement::FencedMechCode(block)
            if !block.config.disabled && block.config.namespace_str == name =>
        {
            *candidate = Some(block.config.namespace);
        }
        SectionElement::Float((element, _)) => {
            collect_interpreter_id_from_element(element, name, candidate);
        }
        _ => {}
    }
}
