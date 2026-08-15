use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{Result, SpecError};

#[derive(Debug)]
pub(crate) struct ExecutorCase {
    pub id: String,
    pub requirements: Vec<String>,
    pub initial_state: String,
    pub committed_state: String,
    resident_program: String,
}

#[derive(Clone, Debug)]
pub(crate) struct CaseManifest {
    pub id: String,
    pub requirements: Vec<String>,
    pub metadata: BTreeMap<String, String>,
    pub resident_program: Option<String>,
}

impl CaseManifest {
    pub(crate) fn metadata(&self, key: &str) -> Result<&str> {
        self.metadata
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| SpecError::new(format!("case {} has no @{key} annotation", self.id)))
    }
}

pub(crate) fn load_manifests(root: &Path) -> Result<Vec<CaseManifest>> {
    let mut paths = Vec::new();
    collect_mec_files(root, &mut paths)?;
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path).map_err(|error| {
                SpecError::new(format!("could not read case {}: {error}", path.display()))
            })?;
            parse_manifest(&source, &path)
        })
        .collect()
}

fn collect_mec_files(directory: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory).map_err(|error| {
        SpecError::new(format!(
            "could not inspect case directory {}: {error}",
            directory.display()
        ))
    })? {
        let entry = entry.map_err(|error| SpecError::new(format!("inspect case: {error}")))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| SpecError::new(format!("inspect {}: {error}", path.display())))?;
        if file_type.is_dir() {
            collect_mec_files(&path, output)?;
        } else if file_type.is_file() && path.extension().is_some_and(|value| value == "mec") {
            output.push(path);
        }
    }
    Ok(())
}

fn parse_manifest(source: &str, path: &Path) -> Result<CaseManifest> {
    let mut metadata = BTreeMap::new();
    for line in source.lines() {
        let trimmed = line.trim();
        let Some(annotation) = trimmed.strip_prefix("-- @") else {
            continue;
        };
        let Some((key, value)) = annotation.split_once(' ') else {
            continue;
        };
        metadata.insert(key.to_string(), value.trim().to_string());
    }
    let id = metadata
        .get("case-id")
        .cloned()
        .ok_or_else(|| SpecError::new(format!("case {} has no @case-id", path.display())))?;
    let requirements: Vec<String> = metadata
        .get("requirements")
        .map(|value| value.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default();
    if requirements.is_empty() {
        return Err(SpecError::new(format!(
            "case {} has no @requirements",
            path.display()
        )));
    }
    let resident_program = source
        .split_once("-- @resident-program")
        .map(|(_, program)| program.trim().to_string())
        .filter(|program| !program.is_empty());
    Ok(CaseManifest {
        id,
        requirements,
        metadata,
        resident_program,
    })
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

    #[test]
    fn parses_generic_case_evidence_inputs() {
        let manifest = parse_manifest(
            "Title\n=====\n-- @case-id benchmark\n-- @requirements BENCH-001\n-- @reference-protocol execution-only\n",
            Path::new("benchmark.mec"),
        )
        .unwrap();
        assert_eq!(manifest.id, "benchmark");
        assert_eq!(manifest.requirements, ["BENCH-001"]);
        assert_eq!(
            manifest.metadata("reference-protocol").unwrap(),
            "execution-only"
        );
        assert!(manifest.resident_program.is_none());
    }
}
