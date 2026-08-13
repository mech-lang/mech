use crate::*;
use mech_core::*;
use mech_engine::{MechProgram, MechProgramConfig, MechProgramEnvironment};
#[cfg(feature = "run")]
use mech_runtime::{FS_LIST, MECH_TOOL_SUBJECT, MechRuntime, RuntimeContext, fs_request};
use nom::{
    IResult,
    branch::alt,
    bytes::complete::tag,
    bytes::complete::{take_until, take_while},
    character::complete::{digit1, space0, space1},
    combinator::{not, opt},
    multi::separated_list1,
};
use std::collections::HashMap;

use bincode::config::standard;
use bincode::serde::encode_to_vec;
use include_dir::{Dir, include_dir};
use std::time::{Duration, Instant};

static DOCS_DIR: Dir = include_dir!("docs");
static EXAMPLES_DIR: Dir = include_dir!("examples/working");

pub enum ReplExecution {
    Output(String),
    Quit,
}

pub struct MechRepl {
    pub docs: Dir<'static>,
    pub examples: Dir<'static>,
    pub active: u64,
    pub programs: HashMap<u64, MechProgram>,
    #[cfg(feature = "run")]
    runtime: Option<MechRuntime>,
    #[cfg(feature = "run")]
    runtime_context: Option<RuntimeContext>,
}

fn repl_error(msg: impl Into<String>) -> MechError {
    MechError::new(GenericError { msg: msg.into() }, None).with_compiler_loc()
}

#[cfg(feature = "run")]
fn has_explicit_source_scheme(specifier: &str) -> bool {
    let Some(colon) = specifier.find(':') else {
        return false;
    };

    if colon == 1 && specifier.as_bytes()[0].is_ascii_alphabetic() {
        return false;
    }

    let scheme = &specifier[..colon];
    let mut chars = scheme.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    first.is_ascii_alphabetic()
        && chars.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '.' | '-')
        })
}

#[cfg(feature = "run")]
fn runtime_repl_load_request(source_path: &str) -> MResult<mech_runtime::SourceRequest> {
    if source_path.is_empty() {
        return Err(repl_error("runtime REPL load path cannot be empty"));
    }

    if has_explicit_source_scheme(source_path) {
        return Ok(mech_runtime::SourceRequest::new(source_path));
    }

    let path = std::path::Path::new(source_path);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    mech_runtime::SourceRequest::from_filesystem_path(path)
}

impl MechRepl {
    pub fn new() -> MechRepl {
        let intrp_id = generate_uuid();
        let retained_program = MechProgram::with_function_catalog(
            MechProgramConfig {
                name: format!("repl-{}", intrp_id),
                environment: MechProgramEnvironment::default(),
            },
            mech_stdlib::source_catalog(),
        );
        let mut programs = HashMap::new();
        programs.insert(intrp_id, retained_program);
        MechRepl {
            active: intrp_id,
            programs,
            docs: DOCS_DIR.clone(),
            examples: EXAMPLES_DIR.clone(),
            #[cfg(feature = "run")]
            runtime: None,
            #[cfg(feature = "run")]
            runtime_context: None,
        }
    }

    pub fn from(retained_program: MechProgram) -> MechRepl {
        let intrp_id = generate_uuid();
        let mut programs = HashMap::new();
        programs.insert(intrp_id, retained_program);
        MechRepl {
            docs: DOCS_DIR.clone(),
            examples: EXAMPLES_DIR.clone(),
            active: intrp_id,
            programs,
            #[cfg(feature = "run")]
            runtime: None,
            #[cfg(feature = "run")]
            runtime_context: None,
        }
    }

    #[cfg(feature = "run")]
    pub fn from_runtime(runtime: MechRuntime) -> MechRepl {
        MechRepl {
            docs: DOCS_DIR.clone(),
            examples: EXAMPLES_DIR.clone(),
            active: 0,
            programs: HashMap::new(),
            runtime: Some(runtime),
            runtime_context: None,
        }
    }

    #[cfg(feature = "run")]
    fn runtime_with_next_turn_context(
        &mut self,
    ) -> MResult<(&mut MechRuntime, &mut RuntimeContext)> {
        if let Some(context) = self.runtime_context.as_mut() {
            context.reset_for_next_turn()?;
        } else {
            let context = self
                .runtime
                .as_ref()
                .ok_or_else(|| repl_error("runtime-backed REPL lost its runtime"))?
                .runtime_context()?;
            self.runtime_context = Some(context);
        }

        match (&mut self.runtime, &mut self.runtime_context) {
            (Some(runtime), Some(context)) => Ok((runtime, context)),
            _ => Err(repl_error("runtime-backed REPL lost its runtime context")),
        }
    }

    #[cfg(feature = "run")]
    pub(crate) fn is_runtime_backed(&self) -> bool {
        self.runtime.is_some()
    }

    #[cfg(feature = "run")]
    pub(crate) fn start_runtime_input_drivers(&mut self) -> MResult<()> {
        self.runtime
            .as_mut()
            .ok_or_else(|| repl_error("runtime-backed REPL lost its runtime"))?
            .start_input_drivers()
    }

    #[cfg(feature = "run")]
    pub(crate) fn drain_runtime_host_inputs(&mut self, max_inputs: usize) -> MResult<usize> {
        self.runtime
            .as_mut()
            .ok_or_else(|| repl_error("runtime-backed REPL lost its runtime"))?
            .drain_host_inputs(max_inputs)
            .map(|outcomes| outcomes.len())
    }

    #[cfg(feature = "run")]
    pub(crate) fn drain_all_pending_runtime_host_inputs(&mut self) -> MResult<usize> {
        let runtime = self
            .runtime
            .as_mut()
            .ok_or_else(|| repl_error("runtime-backed REPL lost its runtime"))?;
        let pending = runtime.pending_host_input_count()?;
        runtime
            .drain_host_inputs(pending)
            .map(|outcomes| outcomes.len())
    }

    #[cfg(feature = "run")]
    pub(crate) fn shutdown_runtime(&mut self) -> MResult<()> {
        self.runtime
            .as_mut()
            .ok_or_else(|| repl_error("runtime-backed REPL lost its runtime"))?
            .shutdown()
    }

    pub fn execute_repl_command_control(
        &mut self,
        repl_cmd: ReplCommand,
    ) -> MResult<ReplExecution> {
        if matches!(repl_cmd, ReplCommand::Quit) {
            return Ok(ReplExecution::Quit);
        }
        self.execute_repl_command(repl_cmd)
            .map(ReplExecution::Output)
    }

    pub fn execute_repl_command(&mut self, repl_cmd: ReplCommand) -> MResult<String> {
        #[cfg(feature = "run")]
        if self.runtime.is_some() {
            return self.execute_runtime_repl_command(repl_cmd);
        }

        let prgrm = self
            .programs
            .get_mut(&self.active)
            .ok_or_else(|| repl_error(format!("active REPL program not found: {}", self.active)))?;

        match repl_cmd {
            ReplCommand::Help => {
                return Ok(help());
            }
            ReplCommand::Quit => {
                return Ok(String::new());
            }
            ReplCommand::Docs(name) => {
                if let Some(name) = name {
                    let glob = format!("*{}*", name);
                    let entries = self.docs.find(&glob).map_err(|error| {
                        repl_error(format!("failed to search documentation: {error}"))
                    })?;
                    for entry in entries {
                        // print out hte contents of hte file
                        match entry.as_file() {
                            Some(file) => match file.contents_utf8() {
                                Some(doc_content) => {
                                    return Ok(format!("{}", doc_content));
                                }
                                None => {
                                    return Ok(format!("No documentation found for {}", name));
                                }
                            },
                            None => {
                                return Ok(format!("No documentation found for {}", name));
                            }
                        }
                    }
                    Ok(format!("No documentation found for {}", name))
                } else {
                    Ok("Enter a doc to search for.".to_string())
                }
            }
            ReplCommand::Symbols(name) => {
                #[cfg(feature = "pretty_print")]
                let out = prgrm.interpreter().pretty_print_symbols();
                #[cfg(not(feature = "pretty_print"))]
                let out = format!("{:#?}", prgrm.interpreter().symbols());
                return Ok(out);
            }
            ReplCommand::Plan => {
                #[cfg(feature = "pretty_print")]
                let out = prgrm.interpreter().plan().pretty_print();
                #[cfg(not(feature = "pretty_print"))]
                let out = format!("{:#?}", prgrm.interpreter().plan());
                return Ok(out);
            }
            ReplCommand::Whos(names) => {
                #[cfg(feature = "whos")]
                {
                    return Ok(whos(prgrm, names));
                }
                #[cfg(not(feature = "whos"))]
                {
                    let _ = names;
                    return Ok("The :whos command requires the whos feature.".to_string());
                }
            }
            ReplCommand::Clear(name) => {
                // Drop the old program and replace it with a new one
                let id = self.active;
                *prgrm = MechProgram::with_function_catalog(
                    MechProgramConfig {
                        name: format!("repl-{}", id),
                        environment: MechProgramEnvironment::default(),
                    },
                    mech_stdlib::source_catalog(),
                );
                return Ok("".to_string());
            }
            ReplCommand::Ls => {
                return Ok(ls());
            }
            ReplCommand::Cd(path) => {
                let path = PathBuf::from(path);
                match env::set_current_dir(&path) {
                    Ok(_) => match env::current_dir() {
                        Ok(current_path) => {
                            return Ok(format!("{}", current_path.display()));
                        }
                        Err(e) => {
                            return Err(MechError::new(
                                PathNotFound {
                                    file_path: path.display().to_string(),
                                },
                                None,
                            )
                            .with_compiler_loc());
                        }
                    },
                    Err(e) => {
                        return Err(MechError::new(
                            PathNotFound {
                                file_path: path.display().to_string(),
                            },
                            None,
                        )
                        .with_compiler_loc());
                    }
                }
            }
            #[cfg(feature = "serde")]
            ReplCommand::Save(path) => {
                let path = PathBuf::from(path);
                let intrp = self.programs.get(&self.active).ok_or_else(|| {
                    repl_error(format!("active REPL program not found: {}", self.active))
                })?;
                let encoded = encode_to_vec(
                    &MechSourceCode::String(format!("{:#?}", intrp.interpreter().plan())),
                    standard(),
                )
                .map_err(|error| {
                    repl_error(format!("failed to encode REPL program state: {error}"))
                })?;
                let mut file = File::create(&path)?;
                file.write_all(&encoded)?;
                return Ok(format!("Saved program state to {}", path.display()));
            }
            ReplCommand::Clc => {
                clc();
                Ok("".to_string())
            }
            ReplCommand::Load(paths) => {
                let mut result = LegacyValue::Empty;
                for source_path in paths {
                    let source = std::fs::read_to_string(&source_path)?;
                    result = prgrm.run_string(&source)?;
                }
                let r = result;
                #[cfg(feature = "pretty_print")]
                let out = r.pretty_print();
                #[cfg(not(feature = "pretty_print"))]
                let out = format!("{:#?}", r);
                return Ok(format!("\n{}\n{}\n", r.kind(), r));
            }
            ReplCommand::Code(code) => {
                let mut result = LegacyValue::Empty;
                for (_, src) in code {
                    result = prgrm.run_string(&src.to_string())?;
                }
                let r = result;
                #[cfg(feature = "pretty_print")]
                let out = r.pretty_print();
                #[cfg(not(feature = "pretty_print"))]
                let out = format!("{:#?}", r);
                let kind_formatted = format!("{}", r.kind()).ansi_color(218);
                return Ok(format!("\n{}\n{}\n", kind_formatted, r));
            }
            ReplCommand::Profile(on) => {
                let _ = on;
                Ok("Profiling is not currently supported in Program.".to_string())
            }
            ReplCommand::Step(step_id, step_count) => {
                let n: u64 = match step_count {
                    Some(n) => n,
                    None => 1,
                };
                let step_id: usize = match step_id {
                    Some(id) => id,
                    None => 0,
                };
                let now = Instant::now();
                let _ = (step_id, n);
                let elapsed_time = now.elapsed();
                return Ok(format!(
                    "Stepping is not currently supported in Program ({})",
                    format_cycles(1, elapsed_time)
                ));
            }
            x => {
                return Err(MechError::new(FeatureNotEnabledError, None).with_compiler_loc());
            }
        }
    }

    #[cfg(feature = "run")]
    fn execute_runtime_repl_command(&mut self, repl_cmd: ReplCommand) -> MResult<String> {
        match repl_cmd {
            ReplCommand::Help => Ok(help()),
            ReplCommand::Quit => Ok(String::new()),
            ReplCommand::Docs(name) => {
                if let Some(name) = name {
                    let glob = format!("*{}*", name);
                    let entries = self.docs.find(&glob).map_err(|error| {
                        repl_error(format!("failed to search documentation: {error}"))
                    })?;
                    for entry in entries {
                        if let Some(file) = entry.as_file() {
                            if let Some(doc_content) = file.contents_utf8() {
                                return Ok(doc_content.to_string());
                            }
                        }
                    }
                    Ok(format!("No documentation found for {}", name))
                } else {
                    Ok("Enter a doc to search for.".to_string())
                }
            }
            ReplCommand::Symbols(_name) => {
                let runtime = self
                    .runtime
                    .as_ref()
                    .ok_or_else(|| repl_error("runtime-backed REPL lost its runtime"))?;
                let mut output = String::new();
                for (name, value) in runtime.root_symbol_values_all()? {
                    output.push_str(&format!("{} = {}\n", name, value));
                }
                Ok(output)
            }
            ReplCommand::Plan => {
                Ok("The :plan command is unavailable through the sealed runtime API.".to_string())
            }
            ReplCommand::Whos(names) => {
                let runtime = self
                    .runtime
                    .as_ref()
                    .ok_or_else(|| repl_error("runtime-backed REPL lost its runtime"))?;
                let rows = if names.is_empty() {
                    runtime.root_symbol_values_all()?
                } else {
                    let name_refs = names.iter().map(String::as_str).collect::<Vec<_>>();
                    runtime.root_symbol_values(&name_refs)?
                };
                let mut output = String::new();
                for (name, value) in rows {
                    output.push_str(&format!("{} = {}\n", name, value));
                }
                Ok(output)
            }
            ReplCommand::Clear(_name) => Err(repl_error(
                "clearing a runtime-backed REPL is not supported; start a new REPL session",
            )),
            ReplCommand::Ls => {
                let current_dir = env::current_dir()?;
                let runtime = self
                    .runtime
                    .as_mut()
                    .ok_or_else(|| repl_error("runtime-backed REPL lost its runtime"))?;
                runtime.check_capability(&fs_request(MECH_TOOL_SUBJECT, FS_LIST, &current_dir)?)?;
                ls_path(&current_dir)
            }
            ReplCommand::Cd(path) => {
                let requested_path = PathBuf::from(path);
                let path = if requested_path.is_absolute() {
                    requested_path
                } else {
                    env::current_dir()?.join(requested_path)
                };
                let runtime = self
                    .runtime
                    .as_mut()
                    .ok_or_else(|| repl_error("runtime-backed REPL lost its runtime"))?;
                runtime.check_capability(&fs_request(MECH_TOOL_SUBJECT, FS_LIST, &path)?)?;
                env::set_current_dir(&path).map_err(|_| {
                    MechError::new(
                        PathNotFound {
                            file_path: path.display().to_string(),
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?;
                env::current_dir()
                    .map(|current_path| current_path.display().to_string())
                    .map_err(Into::into)
            }
            #[cfg(feature = "serde")]
            ReplCommand::Save(_path) => Err(repl_error(
                "saving a runtime-backed REPL is not supported by the sealed runtime API",
            )),
            ReplCommand::Clc => {
                clc();
                Ok(String::new())
            }
            ReplCommand::Load(paths) => {
                let mut result = mech_runtime::RuntimeValueSnapshot::empty();
                for source_path in paths {
                    let request = runtime_repl_load_request(&source_path)?;
                    let (runtime, context) = self.runtime_with_next_turn_context()?;
                    result = runtime
                        .legacy_interpreter()
                        .resolve_and_run_root_module_with_context(
                            context,
                            request,
                            crate::cli::run::cli_module_options(),
                        )?;
                }
                Ok(format!("\n{}\n{}\n", result.kind(), result))
            }
            ReplCommand::Code(code) => {
                let mut result = mech_runtime::RuntimeValueSnapshot::empty();
                for (_, source) in code {
                    let source = source.to_string();
                    let (runtime, context) = self.runtime_with_next_turn_context()?;
                    result = runtime
                        .legacy_interpreter()
                        .run_string_with_context(context, &source)?;
                }
                let kind_formatted = format!("{}", result.kind()).ansi_color(218);
                Ok(format!("\n{}\n{}\n", kind_formatted, result))
            }
            ReplCommand::Profile(on) => {
                let _ = on;
                Ok("Profiling is configured by the retained runtime.".to_string())
            }
            ReplCommand::Step(step_id, step_count) => {
                let _ = (step_id, step_count);
                Ok("Stepping is not currently exposed by the sealed runtime API.".to_string())
            }
            _ => Err(MechError::new(FeatureNotEnabledError, None).with_compiler_loc()),
        }
    }
}

#[cfg(all(test, feature = "run"))]
#[path = "tests/runtime_load.rs"]
mod runtime_load_tests;

fn format_cycles(n: u64, total_duration: Duration) -> String {
    let total_ns = total_duration.as_nanos() as f64;
    let total_s = total_ns / 1_000_000_000.0;

    // Human-friendly total duration
    let formatted_total = if total_ns >= 1_000_000_000.0 {
        format!("{:.3} s", total_s)
    } else if total_ns >= 1_000_000.0 {
        format!("{:.3} ms", total_ns / 1_000_000.0)
    } else if total_ns >= 1_000.0 {
        format!("{:.3} µs", total_ns / 1_000.0)
    } else {
        format!("{:.3} ns", total_ns)
    };

    // Per-cycle duration
    let cycle_ns = total_ns / n as f64;
    let cycle_s = cycle_ns / 1_000_000_000.0;

    let formatted_cycle = if cycle_ns >= 1_000_000_000.0 {
        format!("{:.3} s", cycle_s)
    } else if cycle_ns >= 1_000_000.0 {
        format!("{:.3} ms", cycle_ns / 1_000_000.0)
    } else if cycle_ns >= 1_000.0 {
        format!("{:.3} µs", cycle_ns / 1_000.0)
    } else {
        format!("{:.3} ns", cycle_ns)
    };

    // Cycle frequency
    let freq_hz = 1.0 / cycle_s;
    let formatted_freq = if freq_hz >= 1_000_000_000.0 {
        format!("{:.3} GHz", freq_hz / 1_000_000_000.0)
    } else if freq_hz >= 1_000_000.0 {
        format!("{:.3} MHz", freq_hz / 1_000_000.0)
    } else if freq_hz >= 1_000.0 {
        format!("{:.3} kHz", freq_hz / 1_000.0)
    } else {
        format!("{:.3} Hz", freq_hz)
    };

    format!(
        "{} cycles in {} ({} per cycle, {})",
        n, formatted_total, formatted_cycle, formatted_freq
    )
}
