use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use mech_core::{MResult, MechError};

use super::{ConfigValue, InvalidConfigField};
use crate::{
    BrowserAuthority, BrowserCapabilityGrant, BrowserDomScope, BrowserNetworkScope,
    BrowserOperation, BrowserResource, BrowserResourceKind, BrowserStorageBackend,
    BrowserStorageScope,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechConfigDocument {
    pub source_name: String,
    pub runtime: RuntimeConfigPatch,
    pub serve: Option<ServeHostConfig>,
    pub browser: BrowserAuthority,
    pub capabilities: Vec<ConfigCapabilityGrant>,
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
            browser: BrowserAuthority::default(),
            capabilities: Vec::new(),
        };
        for (key, value) in map {
            match key.as_str() {
                "runtime" => doc.runtime = self.lower_runtime(value)?,
                "serve" => doc.serve = Some(self.lower_serve(value)?),
                "browser" => doc.browser = self.lower_browser(value)?,
                "capabilities" => doc.capabilities = self.lower_capabilities(value)?,
                other => return invalid(format!("unknown top-level config field `{other}`")),
            }
        }
        Ok(doc)
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

    fn lower_browser(&self, value: &ConfigValue) -> MResult<BrowserAuthority> {
        let map = expect_map("browser", value)?;
        let mut authority = BrowserAuthority::default();
        for (key, value) in map {
            match key.as_str() {
                "dom" => self.lower_browser_dom(value, &mut authority)?,
                "clipboard" => self.lower_browser_clipboard(value, &mut authority)?,
                "network" => self.lower_browser_network(value, &mut authority)?,
                "storage" => self.lower_browser_storage(value, &mut authority)?,
                other => return invalid(format!("unknown browser field `{other}`")),
            }
        }
        Ok(authority)
    }

    fn lower_browser_dom(
        &self,
        value: &ConfigValue,
        authority: &mut BrowserAuthority,
    ) -> MResult<()> {
        for (idx, item) in expect_list("browser.dom", value)?.iter().enumerate() {
            let where_ = format!("browser.dom[{idx}]");
            let map = expect_map(&where_, item)?;
            let mut selector = None;
            let mut allow = None;
            for (key, value) in map {
                match key.as_str() {
                    "selector" => {
                        selector = Some(expect_string(&format!("{where_}.selector"), value)?)
                    }
                    "allow" => {
                        allow = Some(expect_browser_operations_for_resource(
                            &format!("{where_}.allow"),
                            value,
                            BrowserResourceKind::Dom,
                        )?)
                    }
                    other => return invalid(format!("unknown browser.dom field `{other}`")),
                }
            }
            let selector =
                selector.ok_or_else(|| invalid_error(format!("{where_}.selector is required")))?;
            let allow =
                allow.ok_or_else(|| invalid_error(format!("{where_}.allow is required")))?;
            let scope = BrowserDomScope::new(selector)
                .map_err(|error| invalid_error(format!("{where_}.selector: {error}")))?;
            authority.grant(BrowserCapabilityGrant {
                resource: BrowserResource::Dom(scope),
                allow,
                budget: None,
            });
        }
        Ok(())
    }

    fn lower_browser_clipboard(
        &self,
        value: &ConfigValue,
        authority: &mut BrowserAuthority,
    ) -> MResult<()> {
        for (idx, item) in expect_list("browser.clipboard", value)?.iter().enumerate() {
            let where_ = format!("browser.clipboard[{idx}]");
            let map = expect_map(&where_, item)?;
            let mut allow = None;
            for (key, value) in map {
                match key.as_str() {
                    "allow" => {
                        allow = Some(expect_browser_operations_for_resource(
                            &format!("{where_}.allow"),
                            value,
                            BrowserResourceKind::Clipboard,
                        )?)
                    }
                    other => return invalid(format!("unknown browser.clipboard field `{other}`")),
                }
            }
            let allow =
                allow.ok_or_else(|| invalid_error(format!("{where_}.allow is required")))?;
            authority.grant(BrowserCapabilityGrant {
                resource: BrowserResource::Clipboard,
                allow,
                budget: None,
            });
        }
        Ok(())
    }

    fn lower_browser_network(
        &self,
        value: &ConfigValue,
        authority: &mut BrowserAuthority,
    ) -> MResult<()> {
        for (idx, item) in expect_list("browser.network", value)?.iter().enumerate() {
            let where_ = format!("browser.network[{idx}]");
            let map = expect_map(&where_, item)?;
            let mut origin = None;
            let mut methods = None;
            let mut allow = None;
            for (key, value) in map {
                match key.as_str() {
                    "origin" => origin = Some(expect_string(&format!("{where_}.origin"), value)?),
                    "methods" => {
                        methods = Some(expect_string_list(&format!("{where_}.methods"), value)?)
                    }
                    "allow" => {
                        allow = Some(expect_browser_operations_for_resource(
                            &format!("{where_}.allow"),
                            value,
                            BrowserResourceKind::Network,
                        )?)
                    }
                    other => return invalid(format!("unknown browser.network field `{other}`")),
                }
            }
            let origin =
                origin.ok_or_else(|| invalid_error(format!("{where_}.origin is required")))?;
            let allow =
                allow.ok_or_else(|| invalid_error(format!("{where_}.allow is required")))?;
            let scope = BrowserNetworkScope::new(origin, methods)
                .map_err(|error| invalid_error(format!("{where_}: {error}")))?;
            authority.grant(BrowserCapabilityGrant {
                resource: BrowserResource::Network(scope),
                allow,
                budget: None,
            });
        }
        Ok(())
    }

    fn lower_browser_storage(
        &self,
        value: &ConfigValue,
        authority: &mut BrowserAuthority,
    ) -> MResult<()> {
        for (idx, item) in expect_list("browser.storage", value)?.iter().enumerate() {
            let where_ = format!("browser.storage[{idx}]");
            let map = expect_map(&where_, item)?;
            let mut backend = None;
            let mut scope = None;
            let mut recursive = None;
            let mut allow = None;
            for (key, value) in map {
                match key.as_str() {
                    "backend" => {
                        let value = expect_string(&format!("{where_}.backend"), value)?;
                        backend = Some(BrowserStorageBackend::parse(&value).ok_or_else(|| {
                            invalid_error(format!("unknown browser.storage backend `{value}`"))
                        })?);
                    }
                    "scope" => scope = Some(expect_string(&format!("{where_}.scope"), value)?),
                    "recursive" => {
                        recursive = Some(expect_bool(&format!("{where_}.recursive"), value)?)
                    }
                    "allow" => {
                        allow = Some(expect_browser_operations_for_resource(
                            &format!("{where_}.allow"),
                            value,
                            BrowserResourceKind::Storage,
                        )?)
                    }
                    other => return invalid(format!("unknown browser.storage field `{other}`")),
                }
            }
            let backend =
                backend.ok_or_else(|| invalid_error(format!("{where_}.backend is required")))?;
            let scope =
                scope.ok_or_else(|| invalid_error(format!("{where_}.scope is required")))?;
            let allow =
                allow.ok_or_else(|| invalid_error(format!("{where_}.allow is required")))?;
            let scope = BrowserStorageScope::new(backend, scope)
                .map_err(|error| invalid_error(format!("{where_}.scope: {error}")))?
                .with_recursive(recursive.unwrap_or(false));
            authority.grant(BrowserCapabilityGrant {
                resource: BrowserResource::Storage(scope),
                allow,
                budget: None,
            });
        }
        Ok(())
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

fn expect_browser_operations_for_resource(
    where_: &str,
    value: &ConfigValue,
    resource: BrowserResourceKind,
) -> MResult<BTreeSet<BrowserOperation>> {
    let operations = expect_string_list(where_, value)?;
    let mut out = BTreeSet::new();
    for operation in operations {
        let parsed = BrowserOperation::parse(&operation)
            .ok_or_else(|| invalid_error(format!("unknown browser operation `{operation}`")))?;
        if !browser_resource_allows_operation(resource, parsed) {
            return invalid(format!(
                "browser {resource:?} grants do not support operation `{parsed}`"
            ));
        }
        out.insert(parsed);
    }
    if out.is_empty() {
        return invalid(format!("{where_} must contain at least one operation"));
    }
    Ok(out)
}

fn browser_resource_allows_operation(
    resource: BrowserResourceKind,
    operation: BrowserOperation,
) -> bool {
    match resource {
        BrowserResourceKind::Dom | BrowserResourceKind::Clipboard => {
            matches!(operation, BrowserOperation::Read | BrowserOperation::Write)
        }
        BrowserResourceKind::Network => matches!(operation, BrowserOperation::Read),
        BrowserResourceKind::Storage => matches!(
            operation,
            BrowserOperation::Read | BrowserOperation::Write | BrowserOperation::List
        ),
    }
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
