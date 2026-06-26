use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use mech_core::{MResult, MechError, ModuleManifestConfig, ModuleManifestExportConfig, ModuleManifestExportKind};
use crate::{HostInstanceConfig, HostManifestConfig, HostContextManifest, RunResourceGrantConfig, validate_run_resource_grant};

use super::{ConfigValue, InvalidConfigField};

#[derive(Clone, Debug, PartialEq)]
pub struct MechConfigDocument {
    pub source_name: String,
    pub runtime: RuntimeConfigPatch,
    pub serve: Option<ServeHostConfig>,
    pub run: Option<RunHostConfig>,
    pub hosts: Vec<HostInstanceConfig>,
    pub capabilities: Vec<ConfigCapabilityGrant>,
    pub host: Option<HostManifestConfig>,
    pub module: Option<ModuleManifestConfig>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeConfigPatch {
    pub name: Option<String>,
    pub limits: RuntimeLimitsPatch,
    pub diagnostics: DiagnosticsConfigPatch,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeLimitsPatch {
    pub max_steps_per_turn: Option<u64>,
    pub max_turn_duration_ms: Option<u64>,
    pub max_memory_bytes: Option<u64>,
    pub max_tasks: Option<u64>,
    pub max_actors: Option<u64>,
    pub max_actor_mailbox_len: Option<u64>,
    pub max_source_bytes: Option<u64>,
    pub max_in_memory_events: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiagnosticsConfigPatch {
    pub trace_enabled: Option<bool>,
    pub profile_enabled: Option<bool>,
    pub debug_enabled: Option<bool>,
    pub log_level: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServeHostConfig {
    pub address: Option<String>,
    pub port: Option<u16>,
    pub paths: Vec<PathBuf>,
    pub stylesheets: Vec<PathBuf>,
    pub shim: Option<PathBuf>,
    pub wasm: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunHostConfig {
    pub paths: Vec<PathBuf>,
    pub grants: Vec<RunResourceGrantConfig>,
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigCapabilityGrant {
    pub kind: ConfigCapabilityKind,
    pub path: PathBuf,
    pub recursive: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigCapabilityKind {
    CapRoot,
    Read,
    Watch,
    Serve,
}

pub struct ConfigLowerer;

impl ConfigLowerer {
    pub fn new() -> Self {
        Self
    }

    pub fn lower(&self, source_name: String, value: ConfigValue) -> MResult<MechConfigDocument> {
        let map = expect_map("config", &value)?;
        let mut doc = MechConfigDocument {
            source_name,
            runtime: RuntimeConfigPatch::default(),
            serve: None,
            run: None,
            hosts: Vec::new(),
            capabilities: Vec::new(),
            host: None,
            module: None,
        };
        for (key, value) in map {
            match key.as_str() {
                "runtime" => doc.runtime = self.lower_runtime(value)?,
                "serve" => doc.serve = Some(self.lower_serve(value)?),
                "run" => doc.run = Some(self.lower_run(value)?),
                "hosts" => doc.hosts = self.lower_hosts(value)?,
                "host" => doc.host = Some(self.lower_host_manifest(value)?),
                "capabilities" => doc.capabilities = self.lower_capabilities(value)?,
                "module" => doc.module = Some(self.lower_module(value)?),
                other => return invalid(format!("unknown top-level config field `{other}`")),
            }
        }
        Ok(doc)
    }

    fn lower_hosts(&self, value: &ConfigValue) -> MResult<Vec<HostInstanceConfig>> {
        let mut out = Vec::new();
        let mut names = BTreeSet::new();
        for (idx, item) in expect_list("hosts", value)?.iter().enumerate() {
            let where_ = format!("hosts[{idx}]");
            let map = expect_map(&where_, item)?;
            let mut name = None;
            let mut provider = None;
            let mut settings = None;
            for (key, value) in map {
                match key.as_str() {
                    "name" => name = Some(expect_string(&format!("{where_}.name"), value)?),
                    "provider" => provider = Some(expect_string(&format!("{where_}.provider"), value)?),
                    "settings" => settings = Some(value.clone()),
                    other => return invalid(format!("unknown {where_} field `{other}`")),
                }
            }
            let name = name.ok_or_else(|| invalid_error(format!("{where_}.name is required")))?;
            if name.trim().is_empty() { return invalid(format!("{where_}.name must be non-empty")); }
            if !names.insert(name.clone()) { return invalid(format!("duplicate host instance `{name}`")); }
            let provider = provider.ok_or_else(|| invalid_error(format!("{where_}.provider is required")))?;
            if provider.trim().is_empty() { return invalid(format!("{where_}.provider must be non-empty")); }
            let settings = settings.unwrap_or_else(|| ConfigValue::Map(BTreeMap::new()));
            out.push(HostInstanceConfig { name, provider, settings });
        }
        Ok(out)
    }

    fn lower_host_manifest(&self, value: &ConfigValue) -> MResult<HostManifestConfig> {
        let map = expect_map("host", value)?;
        let mut provider = None;
        let mut contexts = None;
        for (key, value) in map {
            match key.as_str() {
                "provider" => provider = Some(expect_string("host.provider", value)?),
                "contexts" => contexts = Some(expect_list("host.contexts", value)?.iter().enumerate().map(|(idx, item)| self.lower_host_context(idx, item)).collect::<MResult<Vec<_>>>()?),
                other => return invalid(format!("unknown host field `{other}`")),
            }
        }
        let manifest = HostManifestConfig { provider: provider.ok_or_else(|| invalid_error("host.provider is required"))?, contexts: contexts.ok_or_else(|| invalid_error("host.contexts is required"))? };
        crate::validate_host_manifest(&manifest)?;
        Ok(manifest)
    }

    fn lower_host_context(&self, idx: usize, value: &ConfigValue) -> MResult<HostContextManifest> {
        let where_ = format!("host.contexts[{idx}]");
        let map = expect_map(&where_, value)?;
        let mut name = None;
        let mut base_uri_template = None;
        let mut operations = None;
        for (key, value) in map {
            match key.as_str() {
                "name" => name = Some(expect_string(&format!("{where_}.name"), value)?),
                "base-uri" => base_uri_template = Some(expect_string(&format!("{where_}.base-uri"), value)?),
                "operations" => operations = Some(expect_string_list(&format!("{where_}.operations"), value)?),
                other => return invalid(format!("unknown {where_} field `{other}`")),
            }
        }
        Ok(HostContextManifest { name: name.ok_or_else(|| invalid_error(format!("{where_}.name is required")))?, base_uri_template: base_uri_template.ok_or_else(|| invalid_error(format!("{where_}.base-uri is required")))?, operations: operations.ok_or_else(|| invalid_error(format!("{where_}.operations is required")))? })
    }


    fn lower_module(&self, value: &ConfigValue) -> MResult<ModuleManifestConfig> {
        let map = expect_map("module", value)?;
        let mut name = None;
        let mut exports = None;
        for (key, value) in map {
            match key.as_str() {
                "name" => name = Some(expect_string("module.name", value)?),
                "exports" => {
                    let list = expect_list("module.exports", value)?;
                    exports = Some(list.iter().enumerate().map(|(idx, v)| self.lower_module_export(idx, v)).collect::<MResult<Vec<_>>>()?);
                }
                other => return invalid(format!("unknown module field `{other}`")),
            }
        }
        let name = name.ok_or_else(|| invalid_error("module.name is required"))?;
        if name.trim().is_empty() { return invalid("module.name must be non-empty"); }
        let exports = exports.ok_or_else(|| invalid_error("module.exports is required"))?;
        Ok(ModuleManifestConfig { name, exports })
    }

    fn lower_module_export(&self, idx: usize, value: &ConfigValue) -> MResult<ModuleManifestExportConfig> {
        let where_ = format!("module.exports[{idx}]");
        let map = expect_map(&where_, value)?;
        let mut name = None;
        let mut kind = None;
        let mut base_uri = None;
        let mut operations = None;
        for (key, value) in map {
            match key.as_str() {
                "name" => name = Some(expect_string(&format!("{where_}.name"), value)?),
                "kind" => {
                    let raw = expect_string(&format!("{where_}.kind"), value)?;
                    kind = Some(match raw.as_str() {
                        "context" => ModuleManifestExportKind::Context,
                        _ => return invalid(format!("{where_}.kind must be `context`; got `{raw}`")),
                    });
                }
                "base-uri" => base_uri = Some(expect_string(&format!("{where_}.base-uri"), value)?),
                "operations" => operations = Some(expect_string_list(&format!("{where_}.operations"), value)?),
                other => return invalid(format!("unknown {where_} field `{other}`")),
            }
        }
        let name = name.ok_or_else(|| invalid_error(format!("{where_}.name is required")))?;
        if name.trim().is_empty() { return invalid(format!("{where_}.name must be non-empty")); }
        let kind = kind.ok_or_else(|| invalid_error(format!("{where_}.kind is required")))?;
        let base_uri = base_uri.ok_or_else(|| invalid_error(format!("{where_}.base-uri is required")))?;
        if !base_uri.contains("://") { return invalid(format!("{where_}.base-uri must contain `://`")); }
        let operations = operations.ok_or_else(|| invalid_error(format!("{where_}.operations is required")))?;
        if operations.is_empty() { return invalid(format!("{where_}.operations must contain at least one operation")); }
        for op in &operations {
            if op != "read" && op != "write" {
                return invalid(format!("module context exports only support operations `read` and `write`; got `{op}`"));
            }
        }
        Ok(ModuleManifestExportConfig { name, kind, base_uri, operations })
    }

    fn lower_runtime(&self, value: &ConfigValue) -> MResult<RuntimeConfigPatch> {
        let map = expect_map("runtime", value)?;
        let mut out = RuntimeConfigPatch::default();
        for (key, value) in map {
            match key.as_str() {
                "name" => out.name = Some(expect_string("runtime.name", value)?),
                "limits" => out.limits = self.lower_limits(value)?,
                "diagnostics" => out.diagnostics = self.lower_diagnostics(value)?,
                other => return invalid(format!("unknown runtime field `{other}`")),
            }
        }
        Ok(out)
    }

    fn lower_limits(&self, value: &ConfigValue) -> MResult<RuntimeLimitsPatch> {
        let map = expect_map("runtime.limits", value)?;
        let mut out = RuntimeLimitsPatch::default();
        for (key, value) in map {
            match key.as_str() {
                "max-steps-per-turn" => {
                    out.max_steps_per_turn =
                        Some(expect_u64("runtime.limits.max-steps-per-turn", value)?)
                }
                "max-turn-duration-ms" => {
                    out.max_turn_duration_ms =
                        Some(expect_u64("runtime.limits.max-turn-duration-ms", value)?)
                }
                "max-memory-bytes" => {
                    out.max_memory_bytes =
                        Some(expect_u64("runtime.limits.max-memory-bytes", value)?)
                }
                "max-tasks" => out.max_tasks = Some(expect_u64("runtime.limits.max-tasks", value)?),
                "max-actors" => {
                    out.max_actors = Some(expect_u64("runtime.limits.max-actors", value)?)
                }
                "max-actor-mailbox-len" => {
                    out.max_actor_mailbox_len =
                        Some(expect_u64("runtime.limits.max-actor-mailbox-len", value)?)
                }
                "max-source-bytes" => {
                    out.max_source_bytes =
                        Some(expect_u64("runtime.limits.max-source-bytes", value)?)
                }
                "max-in-memory-events" => {
                    out.max_in_memory_events =
                        Some(expect_u64("runtime.limits.max-in-memory-events", value)?)
                }
                other => return invalid(format!("unknown runtime.limits field `{other}`")),
            }
        }
        Ok(out)
    }

    fn lower_diagnostics(&self, value: &ConfigValue) -> MResult<DiagnosticsConfigPatch> {
        let map = expect_map("runtime.diagnostics", value)?;
        let mut out = DiagnosticsConfigPatch::default();
        for (key, value) in map {
            match key.as_str() {
                "trace-enabled" => {
                    out.trace_enabled =
                        Some(expect_bool("runtime.diagnostics.trace-enabled", value)?)
                }
                "profile-enabled" => {
                    out.profile_enabled =
                        Some(expect_bool("runtime.diagnostics.profile-enabled", value)?)
                }
                "debug-enabled" => {
                    out.debug_enabled =
                        Some(expect_bool("runtime.diagnostics.debug-enabled", value)?)
                }
                "log-level" => {
                    let log_level = expect_string("runtime.diagnostics.log-level", value)?;
                    if !matches!(
                        log_level.as_str(),
                        "error" | "warn" | "info" | "debug" | "trace"
                    ) {
                        return invalid(format!(
                            "runtime.diagnostics.log-level must be one of error, warn, info, debug, trace; got `{log_level}`"
                        ));
                    }
                    out.log_level = Some(log_level)
                }
                other => return invalid(format!("unknown runtime.diagnostics field `{other}`")),
            }
        }
        Ok(out)
    }

    fn lower_serve(&self, value: &ConfigValue) -> MResult<ServeHostConfig> {
        let map = expect_map("serve", value)?;
        let mut out = ServeHostConfig::default();
        for (key, value) in map {
            match key.as_str() {
                "address" => out.address = Some(expect_string("serve.address", value)?),
                "port" => {
                    let port = expect_u64("serve.port", value)?;
                    if !(1..=65535).contains(&port) {
                        return invalid("serve.port must be in 1..65535");
                    }
                    out.port = Some(port as u16);
                }
                "paths" => out.paths = expect_path_list("serve.paths", value)?,
                "stylesheets" => out.stylesheets = expect_path_list("serve.stylesheets", value)?,
                "shim" => out.shim = Some(PathBuf::from(expect_string("serve.shim", value)?)),
                "wasm" => out.wasm = Some(PathBuf::from(expect_string("serve.wasm", value)?)),
                other => return invalid(format!("unknown serve field `{other}`")),
            }
        }
        Ok(out)
    }

    fn lower_run(&self, value: &ConfigValue) -> MResult<RunHostConfig> {
        let map = expect_map("run", value)?;
        let mut out = RunHostConfig::default();

        for (key, value) in map {
            match key.as_str() {
                "paths" => out.paths = expect_path_list("run.paths", value)?,
                "grants" => out.grants = self.lower_run_grants(value)?,
                other => return invalid(format!("unknown run field `{other}`")),
            }
        }

        Ok(out)
    }

    fn lower_run_grants(&self, value: &ConfigValue) -> MResult<Vec<RunResourceGrantConfig>> {
        expect_list("run.grants", value)?.iter().enumerate().map(|(idx, item)| {
            let where_ = format!("run.grants[{idx}]");
            let map = expect_map(&where_, item)?;
            let mut target = None;
            let mut operations = None;
            let mut paths = None;
            for (key, value) in map {
                match key.as_str() {
                    "target" => target = Some(expect_string(&format!("{where_}.target"), value)?),
                    "operations" => operations = Some(expect_string_list(&format!("{where_}.operations"), value)?),
                    "paths" => paths = Some(expect_string_list(&format!("{where_}.paths"), value)?),
                    other => return invalid(format!("unknown {where_} field `{other}`")),
                }
            }
            let grant = RunResourceGrantConfig { target: target.ok_or_else(|| invalid_error(format!("{where_}.target is required")))?, operations: operations.ok_or_else(|| invalid_error(format!("{where_}.operations is required")))?, paths: paths.ok_or_else(|| invalid_error(format!("{where_}.paths is required")))? };
            validate_run_resource_grant(&grant)?;
            Ok(grant)
        }).collect()
    }

    fn lower_capabilities(&self, value: &ConfigValue) -> MResult<Vec<ConfigCapabilityGrant>> {
        let list = expect_list("capabilities", value)?;
        let mut out = Vec::new();
        for (idx, item) in list.iter().enumerate() {
            let where_ = format!("capabilities[{idx}]");
            let map = expect_map(&where_, item)?;
            let mut kind = None;
            let mut path = None;
            let mut recursive = None;
            for (key, value) in map {
                match key.as_str() {
                    "allow" => {
                        kind = Some(
                            match expect_string(&format!("{where_}.allow"), value)?.as_str() {
                                "cap-root" => ConfigCapabilityKind::CapRoot,
                                "read" => ConfigCapabilityKind::Read,
                                "watch" => ConfigCapabilityKind::Watch,
                                "serve" => ConfigCapabilityKind::Serve,
                                other => {
                                    return invalid(format!(
                                        "unknown capability allow value `{other}`"
                                    ));
                                }
                            },
                        )
                    }
                    "path" => {
                        path = Some(PathBuf::from(expect_string(
                            &format!("{where_}.path"),
                            value,
                        )?))
                    }
                    "recursive" => {
                        recursive = Some(expect_bool(&format!("{where_}.recursive"), value)?)
                    }
                    other => return invalid(format!("unknown capability field `{other}`")),
                }
            }
            let Some(kind) = kind else {
                return invalid(format!("{where_}.allow is required"));
            };
            let Some(path) = path else {
                return invalid(format!("{where_}.path is required"));
            };
            out.push(ConfigCapabilityGrant {
                kind,
                path,
                recursive,
            });
        }
        Ok(out)
    }
}

fn expect_map<'a>(
    where_: &str,
    value: &'a ConfigValue,
) -> MResult<&'a BTreeMap<String, ConfigValue>> {
    match value {
        ConfigValue::Map(map) => Ok(map),
        other => invalid(format!(
            "{where_} must be a map/object, got {}",
            type_name(other)
        )),
    }
}

fn expect_list<'a>(where_: &str, value: &'a ConfigValue) -> MResult<&'a Vec<ConfigValue>> {
    match value {
        ConfigValue::List(list) => Ok(list),
        other => invalid(format!("{where_} must be a list, got {}", type_name(other))),
    }
}

fn expect_string(where_: &str, value: &ConfigValue) -> MResult<String> {
    match value {
        ConfigValue::String(s) => Ok(s.clone()),
        other => invalid(format!(
            "{where_} must be a string, got {}",
            type_name(other)
        )),
    }
}

fn expect_bool(where_: &str, value: &ConfigValue) -> MResult<bool> {
    match value {
        ConfigValue::Bool(b) => Ok(*b),
        other => invalid(format!("{where_} must be a bool, got {}", type_name(other))),
    }
}

fn expect_u64(where_: &str, value: &ConfigValue) -> MResult<u64> {
    match value {
        ConfigValue::Integer(i) if *i >= 0 => Ok(*i as u64),
        other => invalid(format!(
            "{where_} must be a non-negative integer, got {}",
            type_name(other)
        )),
    }
}

fn expect_path_list(where_: &str, value: &ConfigValue) -> MResult<Vec<PathBuf>> {
    expect_list(where_, value)?
        .iter()
        .enumerate()
        .map(|(idx, value)| expect_string(&format!("{where_}[{idx}]"), value).map(PathBuf::from))
        .collect()
}

fn expect_string_list(where_: &str, value: &ConfigValue) -> MResult<Vec<String>> {
    expect_list(where_, value)?
        .iter()
        .enumerate()
        .map(|(idx, value)| expect_string(&format!("{where_}[{idx}]"), value))
        .collect()
}

fn invalid_error(reason: impl Into<String>) -> MechError {
    MechError::new(InvalidConfigField::new(reason), None).with_compiler_loc()
}

fn invalid<T>(reason: impl Into<String>) -> MResult<T> {
    Err(invalid_error(reason))
}

fn type_name(value: &ConfigValue) -> &'static str {
    match value {
        ConfigValue::Null => "null",
        ConfigValue::Bool(_) => "bool",
        ConfigValue::Integer(_) => "integer",
        ConfigValue::Float(_) => "float",
        ConfigValue::String(_) => "string",
        ConfigValue::List(_) => "list",
        ConfigValue::Map(_) => "map",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConfigProfileOptions, parse_config_document};

    fn parse(source: &str) -> MResult<MechConfigDocument> {
        parse_config_document("mech.mcfg", source, ConfigProfileOptions::default())
    }

    #[test]
    fn run_paths_parse_and_lower() {
        let doc = parse(
            r#"config := {
  run: {
    paths: ["foo.mec", "bar.mec"]
  }
}
"#,
        )
        .unwrap();

        let run = doc.run.unwrap();
        assert_eq!(
            run.paths,
            vec![PathBuf::from("foo.mec"), PathBuf::from("bar.mec")]
        );
    }

    #[test]
    fn unknown_run_field_fails() {
        let err = parse(
            r#"config := {
  run: {
    bad: true
  }
}
"#,
        )
        .expect_err("unknown run fields must fail");
        let msg = format!("{} {} {:?}", err.kind_name(), err.kind_message(), err);
        assert!(msg.contains("unknown run field `bad`"));
    }

    #[test]
    fn run_cli_stdout_empty_write_parses() {
        let doc = parse(r#"config := { run: { cli: { stdout: { write: [] } } } }"#).unwrap();
        assert_eq!(doc.run.unwrap().cli.stdout.write, Some(vec![]));
    }

    #[test]
    fn run_cli_stdout_line_write_parses() {
        let doc = parse(r#"config := { run: { cli: { stdout: { write: ["line"] } } } }"#).unwrap();
        assert_eq!(doc.run.unwrap().cli.stdout.write, Some(vec!["line".to_string()]));
    }

    #[test]
    fn run_cli_env_path_read_parses() {
        let doc = parse(r#"config := { run: { cli: { env: { read: ["PATH"] } } } }"#).unwrap();
        assert_eq!(doc.run.unwrap().cli.env.read, Some(vec!["PATH".to_string()]));
    }

    #[test]
    fn invalid_run_cli_stdout_path_fails() {
        let err = parse(r#"config := { run: { cli: { stdout: { write: ["html"] } } } }"#)
            .expect_err("invalid stdout path should fail");
        let msg = format!("{} {} {:?}", err.kind_name(), err.kind_message(), err);
        assert!(msg.contains("run.cli.stdout.write contains invalid path `html`"));
    }

    #[test]
    fn invalid_run_cli_env_key_fails() {
        for key in ["HOME/PATH", "1HOME", "HOME-PATH"] {
            let source = format!(r#"config := {{ run: {{ cli: {{ env: {{ read: ["{key}"] }} }} }} }}"#);
            let err = parse(&source).expect_err("invalid env key should fail");
            let msg = format!("{} {} {:?}", err.kind_name(), err.kind_message(), err);
            assert!(msg.contains("run.cli.env.read contains invalid env key"));
        }
    }

    #[test]
    fn unknown_run_cli_field_fails() {
        let err = parse(r#"config := { run: { cli: { prompt: true } } }"#)
            .expect_err("unknown run.cli field should fail");
        let msg = format!("{} {} {:?}", err.kind_name(), err.kind_message(), err);
        assert!(msg.contains("unknown run.cli field `prompt`"));
    }
}
