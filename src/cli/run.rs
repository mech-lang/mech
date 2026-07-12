use clap::{Arg, ArgAction};
use mech_core::*;
use mech_runtime::{
    ConfigValue, HostInstanceConfig, MechRuntime, RunResourceGrantConfig, RuntimeBuilder,
    RuntimeConfig, RuntimeEvent, parse_host_context_target,
};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::Path;

use crate::cli::host_grants;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunInputMode {
    Empty,
    InlineSource(String),
    Paths(Vec<String>),
}

fn is_intended_path(s: &str) -> bool {
    if s.trim().is_empty() {
        return false;
    }

    let path = Path::new(s);
    if path.exists() {
        return true;
    }
    if s.starts_with("./")
        || s.starts_with(".\\")
        || s.starts_with("../")
        || s.starts_with("..\\")
        || s.starts_with('/')
        || s.starts_with('\\')
    {
        return true;
    }
    if s.len() > 2 && s.as_bytes()[1] == b':' {
        return true;
    }
    if s.contains('/') || s.contains('\\') {
        return true;
    }
    if let Some(ext) = path.extension().and_then(OsStr::to_str) {
        match ext {
            // Mech specific
            "mec" | "🤖" | "mecb" | "mdoc" | "mpkg" => true,
            // Data/Standard formats
            "m" | "csv" | "tsv" | "txt" | "md" | "json" | "toml" | "yaml" => true,
            // Web
            "html" | "htm" | "css" | "js" | "wasm" => true,
            // Images
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "bmp" | "ico" => true,
            _ => false,
        }
    } else {
        false
    }
}

pub fn classify_run_inputs(inputs: Vec<String>) -> RunInputMode {
    if inputs.is_empty() {
        return RunInputMode::Empty;
    }

    if inputs.len() == 1 {
        if Path::new(&inputs[0]).exists() {
            return RunInputMode::Paths(inputs);
        }
        if parses_as_executable_run_source(&inputs[0]) {
            return RunInputMode::InlineSource(inputs[0].clone());
        }
        if is_intended_path(&inputs[0]) {
            return RunInputMode::Paths(inputs);
        }
        return RunInputMode::InlineSource(inputs[0].clone());
    }

    let joined = inputs.join(" ");
    if parses_as_executable_run_source(&joined) {
        return RunInputMode::InlineSource(joined);
    }

    if inputs.iter().any(|input| is_intended_path(input)) {
        RunInputMode::Paths(inputs)
    } else {
        RunInputMode::InlineSource(joined)
    }
}

fn parses_as_executable_run_source(input: &str) -> bool {
    mech_syntax::parser::parse(input.trim())
        .map(|program| program_contains_executable_run_source(&program))
        .unwrap_or(false)
}

fn program_contains_executable_run_source(program: &Program) -> bool {
    program.body.sections.iter().any(|section| {
        section
            .elements
            .iter()
            .any(section_element_contains_executable_run_source)
    })
}

fn section_element_contains_executable_run_source(element: &SectionElement) -> bool {
    match element {
        SectionElement::MechCode(codes) => codes
            .iter()
            .any(|(code, _)| mech_code_is_executable_run_source(code)),
        SectionElement::FencedMechCode(fenced) => fenced
            .code
            .iter()
            .any(|(code, _)| mech_code_is_executable_run_source(code)),
        _ => false,
    }
}

fn mech_code_is_executable_run_source(code: &MechCode) -> bool {
    match code {
        MechCode::Statement(_)
        | MechCode::Expression(_)
        | MechCode::FunctionDefine(_)
        | MechCode::FsmImplementation(_)
        | MechCode::FsmSpecification(_)
        | MechCode::Import(_) => true,
        MechCode::Comment(_) | MechCode::Error(_, _) => false,
    }
}

pub fn new_cli_runtime(
    config: RuntimeConfig,
    cli_grants: &host_grants::EffectiveCliHostGrants,
    configured_hosts: &[HostInstanceConfig],
    run_grants: &[RunResourceGrantConfig],
) -> MResult<MechRuntime> {
    for host in configured_hosts {
        if host.name == "cli" && host.provider != "cli" {
            return Err(MechError::new(
                CliRuntimeHostConfigError {
                    reason: format!(
                        "host instance `cli` is reserved for provider `cli` and cannot be configured as provider `{}`",
                        host.provider,
                    ),
                },
                None,
            ));
        }
    }

    let builder = RuntimeBuilder::new().config(config);
    let (mut builder, registered_providers) =
        crate::cli::host_factories::register_cli_host_factories(builder)?;

    let mut saw_cli_instance = false;
    let mut registered_cli_instances = BTreeSet::new();
    for host in configured_hosts {
        if registered_providers.contains(&host.provider) {
            registered_cli_instances.insert(host.name.clone());
            if host.name == "cli" {
                saw_cli_instance = true;
            }
            builder = builder.host_instance(host.clone());
        }
    }

    if !saw_cli_instance {
        registered_cli_instances.insert("cli".to_string());
        builder = builder.host_instance(HostInstanceConfig {
            name: "cli".to_string(),
            provider: "cli".to_string(),
            settings: ConfigValue::Map(std::collections::BTreeMap::new()),
        });
    }

    for grant in cli_grants_to_run_resource_grants(cli_grants) {
        builder = builder.run_resource_grant(grant);
    }

    for grant in run_grants {
        let (instance, _) = parse_host_context_target(&grant.target)?;
        if registered_cli_instances.contains(instance) {
            builder = builder.run_resource_grant(grant.clone());
        }
    }

    builder.build()
}

#[derive(Debug, Clone)]
struct CliRuntimeHostConfigError {
    reason: String,
}

impl MechErrorKind for CliRuntimeHostConfigError {
    fn name(&self) -> &str {
        "CliRuntimeHostConfigError"
    }

    fn message(&self) -> String {
        format!("invalid CLI runtime host config: {}", self.reason)
    }
}

pub fn effective_run_runtime_config(
    loaded_config: Option<&crate::LoadedMechConfig>,
    name: String,
    debug_enabled: bool,
    trace_enabled: bool,
    profile_enabled: bool,
    rounds_per_step: Option<usize>,
) -> MResult<RuntimeConfig> {
    let default_runtime_patch = mech_runtime::RuntimeConfigPatch::default();

    let mut config = crate::apply_runtime_config_patch(
        RuntimeConfig::default(),
        loaded_config
            .as_ref()
            .map(|loaded| &loaded.document.runtime)
            .unwrap_or(&default_runtime_patch),
    )?;

    config.name = name;

    if debug_enabled {
        config.diagnostics.debug_enabled = true;
    }

    if trace_enabled {
        config.diagnostics.trace_enabled = true;
    }

    if profile_enabled {
        config.diagnostics.profile_enabled = true;
    }

    if let Some(rounds_per_step) = rounds_per_step {
        config.limits.max_steps_per_turn = Some(rounds_per_step as u64);
    }

    config.validate()?;
    Ok(config)
}

pub fn run_cli_source_with_events(
    runtime: &mut MechRuntime,
    source: &str,
) -> MResult<(Value, Vec<RuntimeEvent>)> {
    let mut context = runtime.runtime_context()?;
    let result = runtime.run_string_with_context(&mut context, source)?;
    Ok((result, context.events))
}

pub fn run_cli_source_code_with_events(
    runtime: &mut MechRuntime,
    source: &MechSourceCode,
) -> MResult<(Value, Vec<RuntimeEvent>)> {
    let mut context = runtime.runtime_context()?;
    let result = runtime.run_source_with_context(&mut context, source)?;
    Ok((result, context.events))
}

pub fn run_cli_source(runtime: &mut MechRuntime, source: &str) -> MResult<Value> {
    run_cli_source_with_events(runtime, source).map(|(value, _)| value)
}

pub fn run_cli_source_code(runtime: &mut MechRuntime, source: &MechSourceCode) -> MResult<Value> {
    run_cli_source_code_with_events(runtime, source).map(|(value, _)| value)
}

fn cli_grants_to_run_resource_grants(
    grants: &host_grants::EffectiveCliHostGrants,
) -> Vec<RunResourceGrantConfig> {
    let mut out = Vec::new();
    if !grants.env_read_paths.is_empty() {
        out.push(RunResourceGrantConfig {
            target: "cli/env".to_string(),
            operations: vec!["read".to_string()],
            paths: grants.env_read_paths.clone(),
        });
    }
    if !grants.stdout_write_paths.is_empty() {
        out.push(RunResourceGrantConfig {
            target: "cli/stdout".to_string(),
            operations: vec!["write".to_string()],
            paths: grants.stdout_write_paths.clone(),
        });
    }
    if !grants.stderr_write_paths.is_empty() {
        out.push(RunResourceGrantConfig {
            target: "cli/stderr".to_string(),
            operations: vec!["write".to_string()],
            paths: grants.stderr_write_paths.clone(),
        });
    }
    out
}

pub fn cli_host_capability_args() -> Vec<Arg> {
    vec![
        Arg::new("deny_default_capabilities")
            .long("deny-default-capabilities")
            .help("Disable default CLI host capability profiles for this run")
            .global(true)
            .action(ArgAction::SetTrue),
        Arg::new("capabilities")
            .long("capabilities")
            .value_name("CAPABILITY")
            .help("Enable one named CLI host capability profile for this run, e.g. :cli/stdout")
            .global(true)
            .num_args(1)
            .value_parser([":cli/env", ":cli/stdout", ":cli/stderr"])
            .action(ArgAction::Append),
    ]
}

fn cli_host_capability_values(cli_matches: &clap::ArgMatches) -> Vec<String> {
    cli_matches
        .get_many::<String>("capabilities")
        .into_iter()
        .flatten()
        .cloned()
        .collect()
}

pub fn cli_host_capability_selection(
    cli_matches: &clap::ArgMatches,
    _run_matches: Option<&clap::ArgMatches>,
) -> host_grants::CliHostCapabilitySelection {
    let deny_defaults = cli_matches.get_flag("deny_default_capabilities");

    let profiles = cli_host_capability_values(cli_matches);

    host_grants::CliHostCapabilitySelection {
        include_defaults: !deny_defaults,
        profiles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_runtime::{ConfigProfileOptions, parse_config_document};

    #[test]
    fn classifies_single_inline_context_send_with_slashes_as_inline_source() {
        let mode = classify_run_inputs(vec![
            "+> @out := cli/stdout\n@out/line <- \"hi\"".to_string(),
        ]);
        assert!(matches!(mode, RunInputMode::InlineSource(_)));
    }

    #[test]
    fn classifies_single_fenced_context_import_with_slashes_as_inline_source() {
        let mode = classify_run_inputs(vec![
            "```mech
+> @out := cli/stdout
```"
            .to_string(),
        ]);
        assert!(matches!(mode, RunInputMode::InlineSource(_)));
    }

    #[test]
    fn classifies_single_plain_inline_expression_as_inline_source() {
        let mode = classify_run_inputs(vec!["x := 1".to_string()]);
        assert!(matches!(mode, RunInputMode::InlineSource(_)));
    }

    #[test]
    fn classifies_single_formula_with_slash_as_inline_source() {
        let mode = classify_run_inputs(vec!["1 / 2".to_string()]);
        assert!(matches!(mode, RunInputMode::InlineSource(_)));
    }

    #[test]
    fn classifies_split_inline_context_read_with_slashes_as_inline_source() {
        let mode = classify_run_inputs(vec![
            "x".to_string(),
            ":=".to_string(),
            "@env/HOME".to_string(),
        ]);
        assert!(matches!(mode, RunInputMode::InlineSource(_)));
    }

    #[test]
    fn classifies_split_inline_context_send_with_slashes_as_inline_source() {
        let mode = classify_run_inputs(vec![
            "@out/line".to_string(),
            "<-".to_string(),
            "\"hi\"".to_string(),
        ]);
        assert!(matches!(mode, RunInputMode::InlineSource(_)));
    }

    #[test]
    fn classifies_multiple_path_like_inputs_as_paths_even_if_joined_text_parses() {
        let mode = classify_run_inputs(vec!["examples/foo.mec".to_string(), "bar.mec".to_string()]);
        assert!(matches!(mode, RunInputMode::Paths(_)));
    }

    #[test]
    fn new_cli_runtime_filters_non_cli_run_grants() {
        let document = parse_config_document(
            "test.mcfg",
            r#"config := {
  hosts: [
    {name: "cli", provider: "cli", settings: {}}
    {name: "ui", provider: "browser", settings: {}}
  ]
  run: {
    grants: [
      {target: "cli/stdout", operations: ["write"], paths: ["line"]}
      {target: "ui/dom", operations: ["read"], paths: ["counter/_text"]}
    ]
  }
}
"#,
            ConfigProfileOptions::default(),
        )
        .unwrap();
        let run_grants = document.run.as_ref().unwrap().grants.as_slice();
        let mut runtime = new_cli_runtime(
            RuntimeConfig::default(),
            &host_grants::EffectiveCliHostGrants::default(),
            &document.hosts,
            run_grants,
        )
        .unwrap();

        runtime
            .run_string("+> @out := cli/stdout\n@out/line <- \"ok\"\n")
            .unwrap();
    }

    #[test]
    fn new_cli_runtime_keeps_cli_alias_run_grant() {
        let document = parse_config_document(
            "test.mcfg",
            r#"config := {
  hosts: [
    {name: "term", provider: "cli", settings: {}}
  ]
  run: {
    grants: [
      {target: "term/stdout", operations: ["write"], paths: ["line"]}
    ]
  }
}
"#,
            ConfigProfileOptions::default(),
        )
        .unwrap();
        let run_grants = document.run.as_ref().unwrap().grants.as_slice();
        let mut runtime = new_cli_runtime(
            RuntimeConfig::default(),
            &host_grants::EffectiveCliHostGrants::default(),
            &document.hosts,
            run_grants,
        )
        .unwrap();

        runtime
            .run_string("+> @out := term/stdout\n@out/line <- \"ok\"\n")
            .unwrap();
    }

    #[test]
    fn cli_reserved_name_rejects_non_cli_provider() {
        let document = parse_config_document(
            "test.mcfg",
            r#"config := {
  hosts: [
    {name: "cli", provider: "browser", settings: {}}
  ]
  run: {
    grants: [
      {target: "cli/stdout", operations: ["write"], paths: ["line"]}
    ]
  }
}
"#,
            ConfigProfileOptions::default(),
        )
        .unwrap();
        let run_grants = document.run.as_ref().unwrap().grants.as_slice();
        let err = new_cli_runtime(
            RuntimeConfig::default(),
            &host_grants::EffectiveCliHostGrants::default(),
            &document.hosts,
            run_grants,
        )
        .unwrap_err();
        let error = format!("{err:?}");
        assert!(error.contains("cli"), "got {error}");
        assert!(
            error.contains("reserved") || error.contains("provider"),
            "got {error}"
        );
        assert!(error.contains("browser"), "got {error}");
    }

    #[test]
    fn explicit_config_cli_stdout_line_is_authoritative_without_deny_defaults() {
        let document = parse_config_document(
      "test.mcfg",
      r#"config := { run: { grants: [{target: "cli/stdout", operations: ["write"], paths: ["line"]}] } }"#,
      ConfigProfileOptions::default(),
    ).unwrap();
        let loaded = crate::LoadedMechConfig {
            path: std::path::PathBuf::from("test.mcfg"),
            base_dir: std::path::PathBuf::new(),
            document: document.clone(),
            discovered_project_dir: None,
        };
        let cli_grants = host_grants::effective_cli_host_grants(
            Some(&loaded),
            host_grants::CliHostCapabilitySelection::default(),
        )
        .unwrap();
        let run_grants = document.run.as_ref().unwrap().grants.as_slice();
        let mut runtime = new_cli_runtime(
            RuntimeConfig::default(),
            &cli_grants,
            &document.hosts,
            run_grants,
        )
        .unwrap();

        runtime
            .run_string("+> @out := cli/stdout\n@out/line <- \"ok\"\n")
            .unwrap();
        assert!(
            runtime
                .run_string("+> @env := cli/env\nx := @env/HOME\n")
                .is_err()
        );
        assert!(
            runtime
                .run_string("+> @err := cli/stderr\n@err/line <- \"bad\"\n")
                .is_err()
        );
        assert!(
            runtime
                .run_string("+> @out := cli/stdout\n@out/text <- \"bad\"\n")
                .is_err()
        );
    }

    #[test]
    fn explicit_cli_profile_remains_additive_with_config_grants() {
        let document = parse_config_document(
      "test.mcfg",
      r#"config := { run: { grants: [{target: "cli/stdout", operations: ["write"], paths: ["line"]}] } }"#,
      ConfigProfileOptions::default(),
    ).unwrap();
        let loaded = crate::LoadedMechConfig {
            path: std::path::PathBuf::from("test.mcfg"),
            base_dir: std::path::PathBuf::new(),
            document: document.clone(),
            discovered_project_dir: None,
        };
        let cli_grants = host_grants::effective_cli_host_grants(
            Some(&loaded),
            host_grants::CliHostCapabilitySelection {
                include_defaults: true,
                profiles: vec![":cli/stderr".to_string()],
            },
        )
        .unwrap();
        let run_grants = document.run.as_ref().unwrap().grants.as_slice();
        let mut runtime = new_cli_runtime(
            RuntimeConfig::default(),
            &cli_grants,
            &document.hosts,
            run_grants,
        )
        .unwrap();

        runtime
            .run_string("+> @out := cli/stdout\n@out/line <- \"ok\"\n")
            .unwrap();
        runtime
            .run_string("+> @err := cli/stderr\n@err/line <- \"ok\"\n")
            .unwrap();
        assert!(
            runtime
                .run_string("+> @env := cli/env\nx := @env/HOME\n")
                .is_err()
        );
    }
}
