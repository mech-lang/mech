use clap::{Arg, ArgAction};
use mech_core::*;
use mech_host_cli::CliHostFactory;
use mech_runtime::{
  ConfigValue, HostInstanceConfig, MechRuntime, RunResourceGrantConfig, RuntimeBuilder, RuntimeConfig,
  RuntimeEvent, RuntimeEventKind,
};
use std::ffi::OsStr;
use std::path::Path;

use crate::cli::config;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunInputMode {
  Empty,
  InlineSource(String),
  Paths(Vec<String>),
}

fn is_intended_path(s: &str) -> bool {
  if s.trim().is_empty() { return false; }

  let path = Path::new(s);
  if path.exists() {
    return true;
  }
  if s.starts_with("./") || s.starts_with(".\\") ||
    s.starts_with("../") || s.starts_with("..\\") ||
    s.starts_with('/') || s.starts_with('\\') {
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
    section.elements.iter().any(section_element_contains_executable_run_source)
  })
}

fn section_element_contains_executable_run_source(element: &SectionElement) -> bool {
  match element {
    SectionElement::MechCode(codes) => {
      codes.iter().any(|(code, _)| mech_code_is_executable_run_source(code))
    }
    SectionElement::FencedMechCode(fenced) => {
      fenced.code.iter().any(|(code, _)| mech_code_is_executable_run_source(code))
    }
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

fn parses_as_context_addressed_source(input: &str) -> bool {
  mech_syntax::parser::parse(input.trim())
    .map(|program| program_contains_context_addressed_source(&program))
    .unwrap_or(false)
}

fn program_contains_context_addressed_source(program: &Program) -> bool {
  program.body.sections.iter().any(|section| {
    section.elements.iter().any(section_element_contains_context_addressed_source)
  })
}

fn section_element_contains_context_addressed_source(element: &SectionElement) -> bool {
  match element {
    SectionElement::MechCode(codes) => codes.iter().any(|(code, _)| mech_code_contains_context_addressed_source(code)),
    SectionElement::FencedMechCode(fenced) => {
      fenced.code.iter().any(|(code, _)| mech_code_contains_context_addressed_source(code))
    }
    _ => false,
  }
}

fn mech_code_contains_context_addressed_source(code: &MechCode) -> bool {
  match code {
    MechCode::Import(import) => matches!(import.alias, Some(ModuleImportAlias::Context(_))),
    MechCode::Statement(statement) => statement_contains_context_addressed_source(statement),
    MechCode::Expression(expression) => expression_contains_context_addressed_source(expression),
    MechCode::FunctionDefine(function) => {
      function.statements.iter().any(statement_contains_context_addressed_source)
        || function.match_arms.iter().any(|arm| {
          pattern_contains_context_addressed_source(&arm.pattern)
            || expression_contains_context_addressed_source(&arm.expression)
        })
    }
    MechCode::FsmImplementation(fsm) => fsm_contains_context_addressed_source(fsm),
    _ => false,
  }
}

fn fsm_contains_context_addressed_source(fsm: &FsmImplementation) -> bool {
  pattern_contains_context_addressed_source(&fsm.start)
    || fsm.arms.iter().any(|arm| match arm {
      FsmArm::Guard(pattern, guards) => pattern_contains_context_addressed_source(pattern)
        || guards.iter().any(|guard| {
          pattern_contains_context_addressed_source(&guard.condition)
            || guard.transitions.iter().any(transition_contains_context_addressed_source)
        }),
      FsmArm::Transition(pattern, transitions) => pattern_contains_context_addressed_source(pattern)
        || transitions.iter().any(transition_contains_context_addressed_source),
      FsmArm::Comment(_) => false,
    })
}

fn transition_contains_context_addressed_source(transition: &Transition) -> bool {
  match transition {
    Transition::Async(pattern) | Transition::Next(pattern) | Transition::Output(pattern) => {
      pattern_contains_context_addressed_source(pattern)
    }
    Transition::CodeBlock(codes) => codes.iter().any(|(code, _)| mech_code_contains_context_addressed_source(code)),
    Transition::Statement(statement) => statement_contains_context_addressed_source(statement),
  }
}

fn statement_contains_context_addressed_source(statement: &Statement) -> bool {
  match statement {
    Statement::ContextDeclaration(_) | Statement::ContextSend(_) => true,
    Statement::VariableDefine(var_def) => expression_contains_context_addressed_source(&var_def.expression),
    Statement::VariableAssign(assign) => assign.target.context.is_some()
      || assign.target.subscript.as_ref().map(|subs| subs.iter().any(subscript_contains_context_addressed_source)).unwrap_or(false)
      || expression_contains_context_addressed_source(&assign.expression),
    Statement::OpAssign(assign) => assign.target.context.is_some()
      || assign.target.subscript.as_ref().map(|subs| subs.iter().any(subscript_contains_context_addressed_source)).unwrap_or(false)
      || expression_contains_context_addressed_source(&assign.expression),
    Statement::TupleDestructure(tuple) => expression_contains_context_addressed_source(&tuple.expression),
    #[cfg(feature = "invariant_define")]
    Statement::InvariantDefine(invariant) => expression_contains_context_addressed_source(&invariant.expression),
    _ => false,
  }
}

fn pattern_contains_context_addressed_source(pattern: &Pattern) -> bool {
  match pattern {
    Pattern::Expression(expression) => expression_contains_context_addressed_source(expression),
    Pattern::TupleStruct(tuple_struct) => tuple_struct.patterns.iter().any(pattern_contains_context_addressed_source),
    Pattern::Tuple(tuple) => tuple.0.iter().any(pattern_contains_context_addressed_source),
    Pattern::Array(array) => {
      array.prefix.iter().any(pattern_contains_context_addressed_source)
        || array.spread.as_ref().and_then(|spread| spread.binding.as_ref()).map(|binding| pattern_contains_context_addressed_source(binding)).unwrap_or(false)
        || array.suffix.iter().any(pattern_contains_context_addressed_source)
    }
    Pattern::Wildcard => false,
  }
}

fn expression_contains_context_addressed_source(expression: &Expression) -> bool {
  match expression {
    Expression::Var(var) => var.context.is_some(),
    Expression::Slice(slice) => slice.context.is_some() || slice.subscript.iter().any(subscript_contains_context_addressed_source),
    Expression::Formula(factor) => factor_contains_context_addressed_source(factor),
    Expression::FunctionCall(call) => call.args.iter().any(|(_, expression)| expression_contains_context_addressed_source(expression)),
    Expression::FsmPipe(pipe) => pipe.start.args.as_ref().map(|args| args.iter().any(|(_, expression)| expression_contains_context_addressed_source(expression))).unwrap_or(false)
      || pipe.transitions.iter().any(transition_contains_context_addressed_source),
    Expression::Match(match_expr) => expression_contains_context_addressed_source(&match_expr.source)
      || match_expr.arms.iter().any(|arm| pattern_contains_context_addressed_source(&arm.pattern)
        || arm.guard.as_ref().map(expression_contains_context_addressed_source).unwrap_or(false)
        || expression_contains_context_addressed_source(&arm.expression)),
    Expression::Range(range) => range_expression_contains_context_addressed_source(range),
    Expression::Structure(structure) => structure_contains_context_addressed_source(structure),
    Expression::SetComprehension(comp) => expression_contains_context_addressed_source(&comp.expression)
      || comp.qualifiers.iter().any(comprehension_qualifier_contains_context_addressed_source),
    Expression::MatrixComprehension(comp) => expression_contains_context_addressed_source(&comp.expression)
      || comp.qualifiers.iter().any(comprehension_qualifier_contains_context_addressed_source),
    Expression::Literal(_) => false,
  }
}

fn factor_contains_context_addressed_source(factor: &Factor) -> bool {
  match factor {
    Factor::Expression(expression) => expression_contains_context_addressed_source(expression),
    Factor::Negate(factor) | Factor::Not(factor) | Factor::Parenthetical(factor) | Factor::Transpose(factor) => {
      factor_contains_context_addressed_source(factor)
    }
    Factor::Term(term) => factor_contains_context_addressed_source(&term.lhs)
      || term.rhs.iter().any(|(_, factor)| factor_contains_context_addressed_source(factor)),
  }
}

fn structure_contains_context_addressed_source(structure: &Structure) -> bool {
  match structure {
    Structure::Map(map) => map.elements.iter().any(|mapping| expression_contains_context_addressed_source(&mapping.key) || expression_contains_context_addressed_source(&mapping.value)),
    Structure::Matrix(matrix) => matrix.rows.iter().any(|row| row.columns.iter().any(|column| expression_contains_context_addressed_source(&column.element))),
    Structure::Record(record) => record.bindings.iter().any(|binding| expression_contains_context_addressed_source(&binding.value)),
    Structure::Set(set) => set.elements.iter().any(expression_contains_context_addressed_source),
    Structure::Table(table) => table.rows.iter().any(|row| row.columns.iter().any(|column| expression_contains_context_addressed_source(&column.element))),
    Structure::Tuple(tuple) => tuple.elements.iter().any(expression_contains_context_addressed_source),
    Structure::TupleStruct(tuple_struct) => expression_contains_context_addressed_source(&tuple_struct.value),
    Structure::Empty => false,
  }
}

fn subscript_contains_context_addressed_source(subscript: &Subscript) -> bool {
  match subscript {
    Subscript::Brace(subscripts) | Subscript::Bracket(subscripts) => subscripts.iter().any(subscript_contains_context_addressed_source),
    Subscript::Formula(factor) => factor_contains_context_addressed_source(factor),
    Subscript::Range(range) => range_expression_contains_context_addressed_source(range),
    _ => false,
  }
}

fn range_expression_contains_context_addressed_source(range: &RangeExpression) -> bool {
  factor_contains_context_addressed_source(&range.start)
    || range.increment.as_ref().map(|(_, factor)| factor_contains_context_addressed_source(factor)).unwrap_or(false)
    || factor_contains_context_addressed_source(&range.terminal)
}

fn comprehension_qualifier_contains_context_addressed_source(qualifier: &ComprehensionQualifier) -> bool {
  match qualifier {
    ComprehensionQualifier::Generator((pattern, expression)) => pattern_contains_context_addressed_source(pattern)
      || expression_contains_context_addressed_source(expression),
    ComprehensionQualifier::Filter(expression) => expression_contains_context_addressed_source(expression),
    ComprehensionQualifier::Let(var_def) => expression_contains_context_addressed_source(&var_def.expression),
  }
}

pub fn new_cli_runtime(
  config: RuntimeConfig,
  cli_grants: &config::EffectiveCliHostGrants,
) -> MResult<MechRuntime> {
  let mut builder = RuntimeBuilder::new()
    .config(config)
    .host_factory(Box::new(CliHostFactory::new()?))?
    .host_instance(HostInstanceConfig {
      name: "cli".to_string(),
      provider: "cli".to_string(),
      settings: ConfigValue::Map(std::collections::BTreeMap::new()),
    });

  for grant in cli_grants_to_run_resource_grants(cli_grants) {
    builder = builder.run_resource_grant(grant);
  }

  builder.build()
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

fn print_run_runtime_events(events: &[RuntimeEvent]) {
  for event in events {
    match &event.kind {
      RuntimeEventKind::ProgramProfiled { duration_ns, .. } => {
        println!("Cycle Time: {} ns", duration_ns);
      }
      _ => {}
    }
  }
}

pub fn run_cli_source(runtime: &mut MechRuntime, source: &str) -> MResult<Value> {
  let mut context = runtime.runtime_context()?;
  let result = runtime.run_string_with_context(&mut context, source);
  print_run_runtime_events(&context.events);
  result
}

fn cli_grants_to_run_resource_grants(grants: &config::EffectiveCliHostGrants) -> Vec<RunResourceGrantConfig> {
  let mut out = Vec::new();
  if !grants.env_read_paths.is_empty() {
    out.push(RunResourceGrantConfig { target: "cli/env".to_string(), operations: vec!["read".to_string()], paths: grants.env_read_paths.clone() });
  }
  if !grants.stdout_write_paths.is_empty() {
    out.push(RunResourceGrantConfig { target: "cli/stdout".to_string(), operations: vec!["write".to_string()], paths: grants.stdout_write_paths.clone() });
  }
  if !grants.stderr_write_paths.is_empty() {
    out.push(RunResourceGrantConfig { target: "cli/stderr".to_string(), operations: vec!["write".to_string()], paths: grants.stderr_write_paths.clone() });
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
) -> config::CliHostCapabilitySelection {
  let deny_defaults = cli_matches.get_flag("deny_default_capabilities");

  let profiles = cli_host_capability_values(cli_matches);

  config::CliHostCapabilitySelection {
    include_defaults: !deny_defaults,
    profiles,
  }
}

pub fn cli_host_capability_passthrough_values(
  cli_matches: &clap::ArgMatches,
  _run_matches: Option<&clap::ArgMatches>,
) -> Vec<String> {
  Vec::new()
}

#[cfg(test)]
mod tests {
  use super::*;

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
```".to_string(),
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
    let mode = classify_run_inputs(vec![
      "examples/foo.mec".to_string(),
      "bar.mec".to_string(),
    ]);
    assert!(matches!(mode, RunInputMode::Paths(_)));
  }
}
