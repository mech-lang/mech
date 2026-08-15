use std::fs;
use std::path::Path;

use crate::{Result, SpecError};

#[derive(Debug)]
pub(crate) struct ExecutorCase {
    pub id: String,
    pub requirements: Vec<String>,
    pub initial_state: String,
    pub committed_state: String,
    resident_program: String,
}

impl ExecutorCase {
    pub fn load(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path).map_err(|error| {
            SpecError::new(format!("could not read case {}: {error}", path.display()))
        })?;
        Self::parse(&source, path)
    }

    fn parse(source: &str, path: &Path) -> Result<Self> {
        let mut id = None;
        let mut requirements = Vec::new();
        let mut initial_state = None;
        let mut committed_state = None;
        let mut resident_program = String::new();
        let mut reading_program = false;

        for line in source.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("-- @case-id ") {
                id = Some(value.trim().to_string());
                reading_program = false;
                continue;
            }
            if let Some(value) = trimmed.strip_prefix("-- @requirements ") {
                requirements = value.split_whitespace().map(str::to_string).collect();
                reading_program = false;
                continue;
            }
            if let Some(value) = trimmed.strip_prefix("-- @initial-state ") {
                initial_state = Some(value.trim().to_string());
                reading_program = false;
                continue;
            }
            if let Some(value) = trimmed.strip_prefix("-- @committed-state ") {
                committed_state = Some(value.trim().to_string());
                reading_program = false;
                continue;
            }
            if trimmed == "-- @resident-program" {
                reading_program = true;
                continue;
            }
            if reading_program {
                resident_program.push_str(line);
                resident_program.push('\n');
            }
        }

        let required = |value: Option<String>, field: &str| {
            value.ok_or_else(|| SpecError::new(format!("case {} has no @{field}", path.display())))
        };
        let id = required(id, "case-id")?;
        let initial_state = required(initial_state, "initial-state")?;
        let committed_state = required(committed_state, "committed-state")?;
        let resident_program = resident_program.trim().to_string();
        if requirements.is_empty() {
            return Err(SpecError::new(format!(
                "case {} has no @requirements",
                path.display(),
            )));
        }
        if resident_program.is_empty() {
            return Err(SpecError::new(format!(
                "case {} has no @resident-program body",
                path.display(),
            )));
        }
        if initial_state == committed_state {
            return Err(SpecError::new(format!(
                "case {} must use distinct initial and committed states",
                path.display(),
            )));
        }

        Ok(Self {
            id,
            requirements,
            initial_state,
            committed_state,
            resident_program,
        })
    }

    pub fn resident_program(&self) -> &str {
        &self.resident_program
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_case_metadata_and_resident_program() {
        let case = ExecutorCase::parse(
            "Title\n=====\n-- @case-id demo\n-- @requirements RES-001 TURN-004\n-- @initial-state before\n-- @committed-state after\n-- @resident-program\n~state := \"after\"\nresult := state\n",
            Path::new("demo.mec"),
        )
        .unwrap();
        assert_eq!(case.id, "demo");
        assert_eq!(case.requirements, ["RES-001", "TURN-004"]);
        assert_eq!(case.initial_state, "before");
        assert_eq!(case.committed_state, "after");
        assert!(case.resident_program().contains("~state"));
    }
}
