//! Strict bounded sequence of separately pinned existing full-input configs.
use super::super::{config, process};
use serde::Deserialize;
use std::{collections::BTreeSet, ffi::OsStr, fs, path::PathBuf};

pub(super) const ENV: &str = "MECH_CANONICAL_HOSTED_STAGED_INPUT_CONFIG";
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Stage {
    pub name: String,
    pub configuration: config::FileSpec,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    schema: String,
    stages: Vec<Stage>,
}
pub(super) struct Config {
    pub path: PathBuf,
    pub stages: Vec<Stage>,
    pub bytes: Vec<u8>,
}

pub(super) fn exclusive(single: Option<&OsStr>, staged: Option<&OsStr>) -> Result<(), String> {
    if single.is_some() && staged.is_some() {
        return Err(
            "single and staged full-input configuration variables are mutually exclusive".into(),
        );
    }
    Ok(())
}

fn parse(path: PathBuf, bytes: Vec<u8>) -> Result<Config, String> {
    if !path.is_absolute() || path.to_str().is_none() || bytes.len() > 65536 {
        return Err("staged configuration requires an absolute UTF8 path and at most64KiB".into());
    }
    let wire: Wire = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if wire.schema != "mech.canonical-hosted-staged-input-config.v1"
        || !(1..=3).contains(&wire.stages.len())
    {
        return Err("staged configuration requires its exact schema and1..3 stages".into());
    }
    let mut names = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut stages = Vec::new();
    for stage in wire.stages {
        let valid = !stage.name.is_empty()
            && stage.name.len() <= 32
            && stage
                .name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_');
        if !valid || !names.insert(stage.name.clone()) {
            return Err(
                "stage names must be unique1..32 lowercase ASCII letters/digits/-/_".into(),
            );
        }
        let configuration = config::file(stage.configuration)?;
        if !paths.insert(configuration.path.clone()) {
            return Err("duplicate stage configuration path".into());
        }
        stages.push(Stage {
            name: stage.name,
            configuration,
        });
    }
    Ok(Config {
        path,
        stages,
        bytes,
    })
}

pub(super) fn load(value: &OsStr) -> Result<Config, String> {
    let path = PathBuf::from(value);
    if !path.is_absolute() || path.to_str().is_none() {
        return Err("staged configuration path must be absolute UTF8".into());
    }
    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > 65536 {
        return Err("staged configuration must be a regular file within64KiB".into());
    }
    parse(path.clone(), process::small_file(&path)?)
}

pub(super) fn distinct_resolved_paths(
    paths: impl IntoIterator<Item = PathBuf>,
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for path in paths {
        if !seen.insert(path) {
            return Err("stage configuration paths resolve to the same file".into());
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "canonical_hosted_full_stages_config_tests.rs"]
mod tests;
