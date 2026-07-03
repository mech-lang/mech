#![feature(hash_extract_if)]
#![allow(warnings)]
use mech::*;
use mech_core::*;
use mech_syntax::parser;
#[cfg(feature = "run")]
use mech_runtime::{FileSourceResolver, RuntimeConfig, FS_LIST, FS_READ, MECH_TOOL_SUBJECT};
#[cfg(feature = "serve")]
#[cfg(feature = "formatter")]
use mech_syntax::formatter::*;
use std::time::Instant;
use std::fs;
use std::env;
use std::io;

use colored::*;
use std::io::{Write, BufReader, BufWriter, stdout};
use crossterm::{
  ExecutableCommand, QueueableCommand,
  terminal, cursor, style::Print,
};
use ariadne::{Report, ReportKind, Label, Color, sources};
use clap::{Arg, ArgAction, Command};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use tabled::{
  builder::Builder,
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use serde_json;
use std::panic;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "formatter")]
const FORMAT_EXTENSIONS: &[&str] = &["mec", "🤖", "html", "htm", "mdoc"];
#[cfg(any(feature = "formatter", feature = "run"))]
const SKIP_SOURCE_DIRS: &[&str] = &["target", ".git", "dist", "out"];
#[cfg(feature = "run")]
// Keep in sync with read_runtime_source_file_with_capabilities.
const RUN_EXTENSIONS: &[&str] = &["mec", "🤖", "mecb", "mdoc", "mpkg", "m", "csv", "js"];

#[cfg(feature = "run")]
const RUN_DIRECTORY_EXTENSIONS: &[&str] = &["mec", "🤖", "mdoc", "mpkg"];


#[cfg(feature = "build")]
fn is_bytecode_source_path(path: &str) -> bool {
  Path::new(path)
    .extension()
    .and_then(|extension| extension.to_str())
    .map(|extension| extension.eq_ignore_ascii_case("mecb"))
    .unwrap_or(false)
}

#[cfg(feature = "build")]
fn validate_build_bytecode_inputs(paths: &[String]) -> MResult<usize> {
  let bytecode_count = paths.iter().filter(|path| is_bytecode_source_path(path)).count();
  if bytecode_count > 0 && bytecode_count != paths.len() {
    return Err(MechError::new(
      GenericError {
        msg: "Cannot mix bytecode (.mecb) inputs with source inputs in `mech build`; build bytecode inputs separately or rebuild from source.".to_string(),
      },
      None,
    ).with_compiler_loc());
  }
  if bytecode_count > 1 {
    return Err(MechError::new(
      GenericError {
        msg: "Cannot combine multiple bytecode (.mecb) inputs in one `mech build` invocation.".to_string(),
      },
      None,
    ).with_compiler_loc());
  }
  Ok(bytecode_count)
}

#[cfg(any(feature = "formatter", feature = "run"))]
fn source_extension(path: &Path) -> Option<String> {
  path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase())
}

#[cfg(any(feature = "formatter", feature = "run"))]
fn extension_allowed(path: &Path, allowed_extensions: &[&str]) -> bool {
  source_extension(path)
    .map(|ext| allowed_extensions.iter().any(|allowed| *allowed == ext))
    .unwrap_or(false)
}

#[cfg(any(feature = "formatter", feature = "run"))]
fn unsupported_source_path_error(path: &Path, allowed_extensions: &[&str]) -> MechError {
  MechError::new(
    GenericError {
      msg: format!(
        "Unsupported source extension for `{}`; expected one of: {}",
        path.display(),
        allowed_extensions.join(", "),
      ),
    },
    None,
  ).with_compiler_loc()
}

#[cfg(feature = "formatter")]
#[derive(Clone, Debug)]
struct CollectedSourceTarget {
  input_root: PathBuf,
  path: PathBuf,
  relative_path: PathBuf,
  default_output_path: PathBuf,
}


#[cfg(feature = "formatter")]
fn absolute_path(path: &Path) -> MResult<PathBuf> {
  Ok(if path.is_absolute() {
    path.to_path_buf()
  } else {
    std::env::current_dir()?.join(path)
  })
}

#[cfg(feature = "formatter")]
fn normalized_existing_or_absolute(path: &Path) -> MResult<PathBuf> {
  let absolute = absolute_path(path)?;
  Ok(if absolute.exists() {
    absolute.canonicalize()?
  } else {
    absolute
  })
}

#[cfg(feature = "formatter")]
fn paths_equivalent(a: &Path, b: &Path) -> MResult<bool> {
  Ok(normalized_existing_or_absolute(a)? == normalized_existing_or_absolute(b)?)
}

#[cfg(feature = "formatter")]
fn normalize_output_exclusion(output_path: &Path, is_output_file: bool) -> MResult<Option<PathBuf>> {
  if is_output_file {
    return Ok(None);
  }
  let absolute = absolute_path(output_path)?;
  Ok(Some(if absolute.exists() {
    absolute.canonicalize()?
  } else {
    absolute
  }))
}

#[cfg(feature = "formatter")]
fn format_output_exclusion(output_arg: Option<&str>, output_path: &Path, is_output_file: bool) -> MResult<Option<PathBuf>> {
  match output_arg {
    None => Ok(None),
    Some(".") => Ok(None),
    Some(_) if is_output_file => Ok(None),
    Some(_) => {
      let exclusion = normalize_output_exclusion(output_path, false)?;
      match exclusion {
        Some(path) if path == std::env::current_dir()?.canonicalize()? => Ok(None),
        other => Ok(other),
      }
    }
  }
}

#[cfg(feature = "formatter")]
fn format_writes_in_place(output_arg: Option<&str>, output_path: &Path, is_output_file: bool) -> MResult<bool> {
  if is_output_file {
    return Ok(false);
  }
  match output_arg {
    None => Ok(true),
    Some(_) => {
      let cwd = std::env::current_dir()?.canonicalize()?;
      let absolute = absolute_path(output_path)?;
      let normalized = if absolute.exists() {
        absolute.canonicalize()?
      } else {
        absolute
      };
      Ok(normalized == cwd)
    }
  }
}

#[cfg(feature = "formatter")]
fn is_excluded_output_path(path: &Path, output_exclusion: Option<&Path>) -> MResult<bool> {
  let Some(excluded) = output_exclusion else { return Ok(false); };
  let absolute = absolute_path(path)?;
  let normalized = if absolute.exists() {
    absolute.canonicalize()?
  } else {
    absolute
  };
  Ok(normalized == excluded || normalized.starts_with(excluded))
}

#[cfg(feature = "formatter")]
fn explicit_file_relative_path(path: &Path) -> MResult<PathBuf> {
  Ok(path.to_path_buf())
}

#[cfg(feature = "formatter")]
fn safe_output_relative_path(path: &Path) -> MResult<PathBuf> {
  let cwd = std::env::current_dir()?;
  let candidate = if path.is_absolute() {
    match path.strip_prefix(&cwd) {
      Ok(stripped) => stripped.to_path_buf(),
      Err(_) => return Ok(path.file_name().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("output.mec"))),
    }
  } else {
    path.to_path_buf()
  };

  let mut safe = PathBuf::new();
  for component in candidate.components() {
    match component {
      std::path::Component::Normal(part) => safe.push(part),
      std::path::Component::CurDir => {}
      std::path::Component::ParentDir
      | std::path::Component::RootDir
      | std::path::Component::Prefix(_) => {
        return Ok(path.file_name().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("output.mec")));
      }
    }
  }

  if safe.as_os_str().is_empty() {
    Ok(path.file_name().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("output.mec")))
  } else {
    Ok(safe)
  }
}

#[cfg(feature = "formatter")]
fn default_output_relative_path(input_root: &Path, path: &Path) -> MResult<PathBuf> {
  let cwd = std::env::current_dir()?;
  if path.is_relative() {
    return Ok(path.to_path_buf());
  }
  if let Ok(stripped) = path.strip_prefix(&cwd) {
    return Ok(stripped.to_path_buf());
  }
  if let Ok(stripped) = path.strip_prefix(input_root) {
    return Ok(input_root.join(stripped));
  }
  Ok(path.file_name().map(PathBuf::from).unwrap_or_else(|| path.to_path_buf()))
}

#[cfg(feature = "formatter")]
fn read_format_source(path: &Path) -> MResult<MechSourceCode> {
  let extension = source_extension(path).ok_or_else(|| unsupported_source_path_error(path, FORMAT_EXTENSIONS))?;
  match extension.as_str() {
    "mec" | "🤖" | "mdoc" => Ok(MechSourceCode::String(std::fs::read_to_string(path)?)),
    "html" | "htm" => Ok(MechSourceCode::Html(std::fs::read_to_string(path)?)),
    _ => Err(unsupported_source_path_error(path, FORMAT_EXTENSIONS)),
  }
}

#[cfg(feature = "formatter")]
fn skip_directory_format_source(path: &Path, html: bool, writes_in_place: bool) -> bool {
  html && writes_in_place && matches!(source_extension(path).as_deref(), Some("html") | Some("htm"))
}

#[cfg(feature = "run")]
fn skip_directory_run_source(path: &Path) -> bool {
  matches!(source_extension(path).as_deref(), Some("mecb"))
}

#[cfg(feature = "formatter")]
fn collect_format_targets(path: &Path, output_exclusion: Option<&Path>, html: bool, writes_in_place: bool) -> MResult<Vec<CollectedSourceTarget>> {
  if path.is_file() {
    if !extension_allowed(path, FORMAT_EXTENSIONS) {
      return Err(unsupported_source_path_error(path, FORMAT_EXTENSIONS));
    }
    let default_output_path = path.to_path_buf();
    let relative_path = safe_output_relative_path(path)?;
    return Ok(vec![CollectedSourceTarget {
      input_root: path.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
      path: path.to_path_buf(),
      relative_path,
      default_output_path,
    }]);
  }

  if !path.exists() {
    return Err(MechError::new(GenericError { msg: format!("Source path does not exist: {}", path.display()) }, None).with_compiler_loc());
  }

  if !path.is_dir() {
    return Err(MechError::new(GenericError { msg: format!("Source path is neither a file nor directory: {}", path.display()) }, None).with_compiler_loc());
  }

  fn collect_dir(root: &Path, dir: &Path, output_exclusion: Option<&Path>, html: bool, writes_in_place: bool, out: &mut Vec<CollectedSourceTarget>, visited: &mut BTreeSet<PathBuf>) -> MResult<()> {
    if is_excluded_output_path(dir, output_exclusion)? { return Ok(()); }
    let canonical = dir.canonicalize()?;
    if !visited.insert(canonical) { return Ok(()); }
    for entry in fs::read_dir(dir)? {
      let entry = entry?;
      let p = entry.path();
      let file_type = entry.file_type()?;
      if file_type.is_symlink() {
        let target_meta = match fs::metadata(&p) {
          Ok(meta) => meta,
          Err(_) => continue,
        };
        if target_meta.is_dir() {
          continue;
        }
        if target_meta.is_file() && !skip_directory_format_source(&p, html, writes_in_place) && extension_allowed(&p, FORMAT_EXTENSIONS) {
          let relative_path = p.strip_prefix(root).unwrap_or(&p).to_path_buf();
          let default_output_path = default_output_relative_path(root, &p)?;
          out.push(CollectedSourceTarget { input_root: root.to_path_buf(), path: p, relative_path, default_output_path });
        }
        continue;
      }
      if file_type.is_dir() {
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
          if SKIP_SOURCE_DIRS.iter().any(|skip| skip == &name) { continue; }
        }
        if is_excluded_output_path(&p, output_exclusion)? { continue; }
        collect_dir(root, &p, output_exclusion, html, writes_in_place, out, visited)?;
      } else if !skip_directory_format_source(&p, html, writes_in_place) && extension_allowed(&p, FORMAT_EXTENSIONS) {
        let relative_path = p.strip_prefix(root).unwrap_or(&p).to_path_buf();
        let default_output_path = default_output_relative_path(root, &p)?;
        out.push(CollectedSourceTarget { input_root: root.to_path_buf(), path: p, relative_path, default_output_path });
      }
    }
    Ok(())
  }

  let mut out = Vec::new();
  let mut visited = BTreeSet::new();
  collect_dir(path, path, output_exclusion, html, writes_in_place, &mut out, &mut visited)?;
  out.sort_by(|a, b| a.relative_path.cmp(&b.relative_path).then_with(|| a.path.cmp(&b.path)));
  Ok(out)
}

#[cfg(feature = "run")]
fn collect_run_targets(path: &Path) -> MResult<Vec<PathBuf>> {
  let mut ids = mech_runtime::DefaultIdGenerator::new();
  let mut authority = mech_runtime::HostFilesystemAuthority::new(MECH_TOOL_SUBJECT, mech_runtime::SharedCapabilityKernel::new());
  let root = if path.is_dir() { path } else { path.parent().unwrap_or_else(|| Path::new(".")) };
  authority.grant_path(&mut ids, root, true, [FS_READ, FS_LIST])?;
  collect_run_targets_with_capabilities(path, authority.kernel())
}

#[cfg(feature = "run")]
fn collect_run_targets_with_capabilities(path: &Path, kernel: &mech_runtime::SharedCapabilityKernel) -> MResult<Vec<PathBuf>> {
  if path.is_file() {
    let mut kernel = kernel.clone();
    mech_runtime::check_fs_capability(&mut kernel, MECH_TOOL_SUBJECT, FS_READ, path)?;
    if !extension_allowed(path, RUN_EXTENSIONS) {
      return Err(unsupported_source_path_error(path, RUN_EXTENSIONS));
    }
    return Ok(vec![path.to_path_buf()]);
  }
  if !path.exists() {
    return Err(MechError::new(GenericError { msg: format!("Source path does not exist: {}", path.display()) }, None).with_compiler_loc());
  }
  if !path.is_dir() {
    return Err(MechError::new(GenericError { msg: format!("Source path is neither a file nor directory: {}", path.display()) }, None).with_compiler_loc());
  }

  fn collect_dir(dir: &Path, out: &mut Vec<PathBuf>, visited: &mut BTreeSet<PathBuf>, kernel: &mech_runtime::SharedCapabilityKernel) -> MResult<()> {
    let mut check_kernel = kernel.clone();
    mech_runtime::check_fs_capability(&mut check_kernel, MECH_TOOL_SUBJECT, FS_LIST, dir)?;
    let canonical = dir.canonicalize()?;
    if !visited.insert(canonical) { return Ok(()); }
    for entry in fs::read_dir(dir)? {
      let entry = entry?;
      let p = entry.path();
      let file_type = entry.file_type()?;
      if file_type.is_symlink() {
        let target_meta = match fs::metadata(&p) {
          Ok(meta) => meta,
          Err(_) => continue,
        };
        if target_meta.is_dir() {
          continue;
        }
        if target_meta.is_file() && !skip_directory_run_source(&p) && extension_allowed(&p, RUN_DIRECTORY_EXTENSIONS) {
          out.push(p);
        }
        continue;
      }
      if file_type.is_dir() {
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
          if SKIP_SOURCE_DIRS.iter().any(|skip| skip == &name) { continue; }
        }
        collect_dir(&p, out, visited, kernel)?;
      } else if !skip_directory_run_source(&p) && extension_allowed(&p, RUN_DIRECTORY_EXTENSIONS) {
        out.push(p);
      }
    }
    Ok(())
  }

  let mut out = Vec::new();
  let mut visited = BTreeSet::new();
  collect_dir(path, &mut out, &mut visited, kernel)?;
  out.sort();
  Ok(out)
}

#[cfg(feature = "formatter")]
fn format_output_matches_input_dir(mech_paths: &[String], output_path: &Path, is_output_file: bool) -> MResult<bool> {
  if is_output_file {
    return Ok(false);
  }
  for input in mech_paths {
    let input_path = Path::new(input);
    if input_path.is_dir() && paths_equivalent(input_path, output_path)? {
      return Ok(true);
    }
  }
  Ok(false)
}

#[cfg(feature = "formatter")]
fn reject_ambiguous_matching_output_dir(output_matches_input_dir: bool, input_count: usize, output_path: &Path) -> MResult<()> {
  if output_matches_input_dir && input_count > 1 {
    return Err(MechError::new(
      GenericError {
        msg: format!(
          "Output directory `{}` matches one of multiple format inputs. Use in-place formatting without --out, or choose a distinct output directory.",
          output_path.display(),
        ),
      },
      None,
    ).with_compiler_loc());
  }
  Ok(())
}

#[cfg(feature = "formatter")]
fn format_output_file_for_target(
  target: &CollectedSourceTarget,
  output_path: &Path,
  is_output_file: bool,
  writes_in_place: bool,
  html: bool,
) -> PathBuf {
  let mut path = if is_output_file {
    output_path.to_path_buf()
  } else if writes_in_place {
    target.default_output_path.clone()
  } else {
    output_path.join(&target.relative_path)
  };
  if html && !is_output_file {
    path = path.with_extension("html");
  }
  path
}

#[cfg(feature = "formatter")]
fn ensure_unique_format_outputs(targets: &[CollectedSourceTarget], output_path: &Path, is_output_file: bool, writes_in_place: bool, html: bool) -> MResult<()> {
  let mut seen: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
  for target in targets {
    let output_file = format_output_file_for_target(target, output_path, is_output_file, writes_in_place, html);
    if let Some(previous) = seen.insert(output_file.clone(), target.path.clone()) {
      return Err(MechError::new(
        GenericError {
          msg: format!(
            "Format output collision for `{}` between `{}` and `{}`",
            output_file.display(),
            previous.display(),
            target.path.display(),
          ),
        },
        None,
      ).with_compiler_loc());
    }
  }
  Ok(())
}

#[cfg(feature = "formatter")]
fn reject_multi_target_file_output(target_count: usize, output_path: &Path, is_output_file: bool) -> MResult<()> {
  if is_output_file && target_count > 1 {
    return Err(MechError::new(
      GenericError {
        msg: format!(
          "Cannot write {} formatted sources into single output file `{}`. Use an output directory instead.",
          target_count,
          output_path.display(),
        ),
      },
      None,
    ).with_compiler_loc());
  }
  Ok(())
}

#[cfg(has_file_wasm)]
static MECHWASM: &[u8] = include_bytes!("../../src/wasm/pkg/mech_wasm_bg.wasm.br");
#[cfg(not(has_file_wasm))]
static MECHWASM: &[u8] = b"No Embedded WASM";

#[cfg(has_file_js)]
static MECHJS: &[u8] = include_bytes!("../../src/wasm/pkg/mech_wasm.js");
#[cfg(not(has_file_js))]
static MECHJS: &[u8] = b"No Embedded JS";

#[cfg(has_file_shim)]
static SHIMHTML: &str = include_str!("../../include/index.html");
#[cfg(not(has_file_shim))]
static SHIMHTML: &str = "No Embedded Shim";

#[cfg(has_file_stylesheet)]
static STYLESHEET: &str = include_str!("../../include/style.css");
#[cfg(not(has_file_stylesheet))]
static STYLESHEET: &str = "No Embedded Stylesheet";

#[derive(Debug, Clone)]
pub struct Utf8ConversionError {
  pub source_error: String
}
impl MechErrorKind for Utf8ConversionError {
  fn name(&self) -> &str {
    "Utf8ConversionError"
  }
  fn message(&self) -> String {
    format!("Failed to convert bytes into UTF-8 string: {}", self.source_error)
  }
}

#[cfg(feature = "bundle_web")]
use mech::cli::bundle_web;
#[cfg(any(feature = "serve", feature = "run"))]
use mech::cli::capabilities;

#[cfg(any(feature = "serve", feature = "run"))]
use mech::cli::config;
#[cfg(feature = "run")]
use mech::cli::run::{
  classify_run_inputs, cli_host_capability_args, cli_host_capability_passthrough_values,
  cli_host_capability_selection, effective_run_runtime_config, new_cli_runtime, run_cli_source, run_cli_source_code,
  RunInputMode,
};

#[cfg(feature = "run")]
fn add_cli_host_capability_args(command: Command) -> Command {
  command.args(cli_host_capability_args())
}

#[cfg(not(feature = "run"))]
fn add_cli_host_capability_args(command: Command) -> Command {
  command
}

#[cfg(feature = "run")]
fn add_run_subcommand(command: Command) -> Command {
  command.subcommand(Command::new("run")
    .about("Run Mech source files, project inputs, or inline Mech code.")
    .arg(Arg::new("mech_run_paths")
      .help("Source .mec files, project folders, or inline Mech code.")
      .required(false)
      .action(ArgAction::Append))
    .arg(Arg::new("debug")
      .short('d')
      .long("debug")
      .help("Print debug info")
      .action(ArgAction::SetTrue))
    .arg(Arg::new("time")
      .short('t')
      .long("time")
      .help("Measure how long the program takes to execute.")
      .action(ArgAction::SetTrue))
    .arg(Arg::new("rounds-per-step")
      .long("rounds-per-step")
      .value_name("ROUNDS")
      .help("Sets the number of rounds per step. Overrides runtime.limits.max-steps-per-turn.")
      .required(false))
    .arg(Arg::new("trace")
      .long("trace")
      .help("Print trace output for state-machine arms and function calls")
      .action(ArgAction::SetTrue)))
}

#[cfg(not(feature = "run"))]
fn add_run_subcommand(command: Command) -> Command {
  command
}

#[cfg(feature = "serve")]
fn add_serve_subcommand(command: Command) -> Command {
  command.subcommand(Command::new("serve")
      .about("Serve Mech program over an HTTP server.")
      .arg(Arg::new("mech_serve_file_paths")
        .help("Source .mec files, .mecb bytecode files, project folders, or directories")
        .required(false)
        .action(ArgAction::Append))
      .arg(Arg::new("port")
        .short('p')
        .long("port")
        .value_name("PORT")
        .help("Sets the port for the server (8081)"))
      .arg(Arg::new("stylesheet")
        .short('s')
        .long("stylesheet")
        .value_name("STYLESHEET")
        .num_args(1..)
        .action(ArgAction::Append)
        .help("Sets the stylesheet for the HTML output"))
      .arg(Arg::new("shim")
        .short('m')
        .long("shim")
        .value_name("SHIM")
        .help("Sets the shim for the HTML output"))
      .arg(Arg::new("wasm")
        .short('w')
        .long("wasm")
        .value_name("WASM")
        .help("Sets the the path to the wasm package"))
      .arg(Arg::new("address")
        .short('a')
        .long("address")
        .value_name("ADDRESS")
        .help("Sets the address of the server (127.0.0.1)"))
      .args(host_delegation_args()))
}

#[cfg(not(feature = "serve"))]
fn add_serve_subcommand(command: Command) -> Command {
  command
}

async fn load_stylesheets(paths: &[String], fallback_url: &str) -> Result<String, MechError> {
  if paths.is_empty() {
    let stylesheet = read_or_download("", fallback_url, Some(STYLESHEET.as_bytes())).await?;
    return String::from_utf8(stylesheet)
      .map_err(|e| MechError::new(Utf8ConversionError { source_error: e.to_string() }, None).with_compiler_loc());
  }

  let mut combined = String::new();
  for path in paths {
    let stylesheet = match std::fs::read(path) {
      Ok(content) => {
        println!("Using stylesheet: {}", path);
        content
      }
      Err(_) => {
        println!("\nStylesheet not found:\n  {}", path);
        read_or_download("", fallback_url, Some(STYLESHEET.as_bytes())).await?
      }
    };
    let stylesheet_str = String::from_utf8(stylesheet)
      .map_err(|e| MechError::new(Utf8ConversionError { source_error: e.to_string() }, None).with_compiler_loc())?;
    if !combined.is_empty() {
      combined.push('\n');
    }
    combined.push_str(&stylesheet_str);
  }
  Ok(combined)
}

#[tokio::main]
async fn main() -> Result<(), MechError> {
  /*panic::set_hook(Box::new(|panic_info| {
    // do nothing.
  }));*/

  let text_logo = r#"
  ┌─────────┐ ┌──────┐ ┌─┐ ┌──┐ ┌─┐  ┌─┐
  └───┐ ┌───┘ └──────┘ │ │ └┐ │ │ │  │ │
  ┌─┐ │ │ ┌─┐ ┌──────┐ │ │  └─┘ │ └─┐│ │
  │ │ │ │ │ │ │ ┌────┘ │ │  ┌─┐ │ ┌─┘│ │
  │ │ └─┘ │ │ │ └────┐ │ └──┘ │ │ │  │ │
  └─┘     └─┘ └──────┘ └──────┘ └─┘  └─┘"#.truecolor(246,192,78);


  let super_3D_logo = r#"
          _____                      _____                     _____                     _____
         ╱╲    ╲                    ╱╲    ╲                   ╱╲    ╲                   ╱╲    ╲
        ╱┊┊╲    ╲                  ╱┊┊╲    ╲                 ╱┊┊╲____╲                 ╱┊┊╲____╲
        ╲┊┊┊╲    ╲                 ╲┊┊┊╲    ╲               ╱┊┊┊╱    ╱                ╱┊┊┊╱    ╱
      ___╲┊┊┊╲    ╲              ___╲┊┊┊╲    ╲             ╱┊┊┊╱   _╱___             ╱┊┊┊╱    ╱
     ╱╲   ╲┊┊┊╲    ╲            ╱╲   ╲┊┊┊╲    ╲           ╱┊┊┊╱   ╱╲    ╲           ╱┊┊┊╱    ╱
    ╱┊┊╲___╲┊┊┊╲    ╲          ╱┊┊╲   ╲┊┊┊╲    ╲         ╱┊┊┊╱   ╱┊┊╲    ╲         ╱┊┊┊╱___ ╱
   ╱┊┊┊╱   ╱┊┊┊┊╲    ╲        ╱┊┊┊┊╲   ╲┊┊┊╲    ╲       ╱┊┊┊╱    ╲┊┊┊╲    ╲       ╱┊┊┊┊╲    ╲   _____
  ╱┊┊┊╱   ╱┊┊┊┊┊┊╲    ╲      ╱┊┊┊┊┊┊╲   ╲┊┊┊╲    ╲     ╱┊┊┊╱    ╱ ╲┊┊┊╲    ╲     ╱┊┊┊┊┊┊╲    ╲ ╱╲    ╲
 ╱┊┊┊╱   ╱┊┊┊╱╲┊┊┊╲    ╲    ╱┊┊┊╱╲┊┊┊╲   ╲┊┊┊╲____╲   ╱┊┊┊╱    ╱   ╲┊┊┊╲____╲   ╱┊┊┊╱╲┊┊┊╲____╱┊┊╲____╲
╱┊┊┊╱   ╱┊┊┊╱  ╲┊┊┊╲____╲  ╱┊┊┊╱__╲┊┊┊╲   ╲┊┊╱    ╱  ╱┊┊┊╱____╱    ╱┊┊┊╱    ╱  ╱┊┊┊╱  ╲┊┊╱   ╱┊┊┊╱    ╱
╲┊┊╱   ╱┊┊┊╱    ╲┊┊╱    ╱  ╲┊┊┊╲   ╲┊┊┊╲   ╲╱____╱   ╲┊┊┊╲    ╲    ╲┊┊╱    ╱   ╲┊┊╱    ╲╱___╱┊┊┊╱    ╱
 ╲╱___╱┊┊┊╱   ___╲╱____╱    ╲┊┊┊╲   ╲┊┊┊╲    ╲        ╲┊┊┊╲    ╲    ╲╱____╱     ╲╱____╱    ╱┊┊┊╱    ╱
     ╱┊┊┊╱   ╱╲    ╲         ╲┊┊┊╲   ╲┊┊┊╲____╲        ╲┊┊┊╲    ╲____                     ╱┊┊┊╱    ╱
     ╲┊┊╱   ╱┊┊╲____╲         ╲┊┊┊╲   ╲┊┊╱    ╱         ╲┊┊┊╲  ╱╲    ╲                   ╱┊┊┊╱    ╱
      ╲╱___╱┊┊┊╱    ╱          ╲┊┊┊╲   ╲╱____╱           ╲┊┊┊╲╱┊┊╲____╲                 ╱┊┊┊╱    ╱
          ╱┊┊┊╱    ╱            ╲┊┊┊╲    ╲                ╲┊┊┊┊┊┊╱    ╱                ╱┊┊┊╱    ╱
         ╱┊┊┊╱    ╱              ╲┊┊┊╲____╲                ╲┊┊┊┊╱    ╱                ╱┊┊┊╱    ╱
        ╱┊┊┊╱    ╱                ╲┊┊╱    ╱                 ╲┊┊╱    ╱                 ╲┊┊╱    ╱
        ╲┊┊╱    ╱                  ╲╱____╱                   ╲╱____╱                   ╲╱____╱
         ╲╱____╱"#.truecolor(246,192,78);


  let micromika = "╭◉╮".truecolor(246,192,78);
  let micromika_point = "╭◉─".truecolor(246,192,78);
  let micromika_hello = "╭◉╯".truecolor(246,192,78);
  let help_cmd = ":help".bright_yellow();
  let quit_cmd = ":quit".bright_yellow();
  let ctrlc_cmd = ":ctrl+c".bright_yellow();
  let mika_open = "⸢".bright_yellow();
  let mika_close = "⸥".bright_yellow();

  let about = format!("{}", text_logo);

  let cli_command = Command::new("Mech")
    .subcommand_precedence_over_arg(true)
    .version(VERSION)
    .author("Corey Montella corey@mech-lang.org")
    .about(about)
    .arg(Arg::new("mech_paths")
        .help("Source .mec files, .mecb bytecode files, project folders, or inline Mech code")
        .required(false)
        .action(ArgAction::Append))
    .arg(Arg::new("debug")
        .short('d')
        .long("debug")
        .help("Print debug info")
        .action(ArgAction::SetTrue))
    .subcommand(Command::new("format")
      .about("Format Mech source code into standard format.")
      .arg(Arg::new("mech_format_file_paths")
        .help("Source .mec/.mdoc files, HTML files, or directories")
        .required(false)
        .action(ArgAction::Append))
      .arg(Arg::new("output_path")
        .short('o')
        .long("out")
        .help("Destination folder.")
        .required(false))
      .arg(Arg::new("stylesheet")
        .short('s')
        .long("stylesheet")
        .value_name("STYLESHEET")
        .num_args(1..)
        .action(ArgAction::Append)
        .help("Sets the stylesheet for the HTML output"))
      .arg(Arg::new("shim")
        .short('m')
        .long("shim")
        .value_name("SHIM")
        .help("Sets the shim for the HTML output"))
      .arg(Arg::new("html")
        .short('t')
        .long("html")
        .required(false)
        .help("Output as HTML")
        .action(ArgAction::SetTrue)))
    .subcommand(Command::new("build")
      .about("Build Mech program into a binary.")
      .arg(Arg::new("mech_build_file_paths")
        .help("Source .mec and .mecb files")
        .required(false)
        .action(ArgAction::Append))
      .arg(Arg::new("debug")
        .short('d')
        .long("debug")
        .help("Print debug info")
        .action(ArgAction::SetTrue))
      .arg(Arg::new("output_path")
        .short('o')
        .long("out")
        .help("Destination folder.")
        .required(false)))
    .subcommand(Command::new("test")
      .about("Validate program invariants.")
      .arg(Arg::new("mech_test_file_paths")
        .help("Source .mec and .mecb files or directories")
        .required(false)
        .action(ArgAction::Append))
      .arg(Arg::new("output_path")
        .short('o')
        .long("out")
        .help("Write test output to .json or .mec.")
        .required(false))
      .arg(Arg::new("verbose")
        .short('v')
        .long("verbose")
        .help("Print verbose pass/fail details.")
        .action(ArgAction::SetTrue)
        .required(false)))
    .arg(Arg::new("tree")
        .short('e')
        .long("tree")
        .help("Print parse tree")
        .action(ArgAction::SetTrue))
    .arg(Arg::new("time")
        .short('t')
        .long("time")
        .help("Measure how long the programs takes to execute.")
        .action(ArgAction::SetTrue))
    .arg(Arg::new("rounds-per-step")
        .long("rounds-per-step")
        .value_name("ROUNDS")
        .help("Sets the number of rounds per step (10_000)")
        .required(false))
    .arg(Arg::new("trace")
        .long("trace")
        .help("Print trace output for state-machine arms and function calls")
        .action(ArgAction::SetTrue))
    .arg(Arg::new("repl")
        .short('r')
        .long("repl")
        .help("Start REPL")
        .action(ArgAction::SetTrue));

  let cli_command = add_run_subcommand(cli_command);
  let cli_command = add_serve_subcommand(cli_command);
  let cli_command = add_cli_host_capability_args(cli_command);

  #[cfg(feature = "bundle_web")]
  let cli_command = cli_command.subcommand(bundle_web::bundle_web_command());

  #[cfg(any(feature = "serve", feature = "run"))]
  let cli_command = capabilities::add_filesystem_capability_args(cli_command);

  #[cfg(any(feature = "serve", feature = "run"))]
  let cli_command = config::add_config_args(cli_command);

  #[cfg(all(feature = "bundle_web", not(feature = "serve")))]
  let cli_command = bundle_web::add_config_args(cli_command);

  let cli_matches = cli_command.get_matches();

  let debug_flag = cli_matches.get_flag("debug");
  let tree_flag = cli_matches.get_flag("tree");
  let mut repl_flag = cli_matches.get_flag("repl");
  let time_flag = cli_matches.get_flag("time");
  let trace_flag = cli_matches.get_flag("trace");
  let root_rounds_per_step = cli_matches.get_one::<String>("rounds-per-step").and_then(|s| s.parse::<usize>().ok());

  let shim_backup_url = "https://raw.githubusercontent.com/mech-lang/mech/refs/heads/main/include/shim.html".to_string();
  let stylesheet_backup_url = "https://raw.githubusercontent.com/mech-lang/mech/refs/heads/main/include/style.css".to_string();
  let wasm_backup_url = format!("https://github.com/mech-lang/mech/releases/download/v{}-beta/mech_wasm_bg.wasm.br", VERSION);
  let js_backup_url = format!("https://github.com/mech-lang/mech/releases/download/v{}-beta/mech_wasm.js", VERSION);

  // --------------------------------------------------------------------------
  // Bundle Web
  // --------------------------------------------------------------------------
  #[cfg(feature = "bundle_web")]
  if let Some(bundle_matches) = cli_matches.subcommand_matches("bundle-web") {
    let badge = "[Mech Bundle]".truecolor(34, 204, 187);

    let loaded = bundle_web::load_bundle_web_config(bundle_matches)?;
    println!("{badge} Loading config… {}", loaded.path.display());

    let options = bundle_web::effective_bundle_web_options(bundle_matches, loaded)?;
    let result = mech::bundle_web_project(options)?;

    println!("{badge} Bundle written: {}", result.output_dir.display());
    println!("{badge} Sources bundled: {}", result.source_count);
    return Ok(());
  }

  // --------------------------------------------------------------------------
  // Serve
  // --------------------------------------------------------------------------
  #[cfg(feature = "serve")]
  if let Some(serve_matches) = cli_matches.subcommand_matches("serve") {
    let badge = "[Mech Server]".truecolor(34, 204, 187);
    let error_badge = "[Error]".truecolor(246, 98, 78);

    let loaded_config = config::load_cli_config(serve_matches)?;
    let effective = config::effective_serve_options(serve_matches, loaded_config.as_ref())?;
    let default_runtime_patch = mech_runtime::RuntimeConfigPatch::default();
    let runtime_config = mech::apply_runtime_config_patch(
      mech_runtime::RuntimeConfig::default(),
      loaded_config
        .as_ref()
        .map(|loaded| &loaded.document.runtime)
        .unwrap_or(&default_runtime_patch),
    )?;
    let host_config = loaded_config
      .as_ref()
      .map(|loaded| mech::web_runtime_injection_config_from_document(
        &loaded.document,
        &runtime_config,
      ))
      .transpose()?;
    let config_shim_at_root = loaded_config
      .as_ref()
      .and_then(|loaded| loaded.document.serve.as_ref())
      .and_then(|serve| serve.shim.as_ref())
      .is_some()
      && serve_matches.get_one::<String>("shim").is_none();
    if let Some(loaded) = loaded_config.as_ref() {
      println!("{badge} Loaded host config entries: {}", loaded.document.hosts.len());
    }

    let full_address = format!("{}:{}", effective.address, effective.port);
    #[cfg(feature = "host_delegation_signing")]
    let host_config_injection = serve_host_delegation_injection(
      serve_matches,
      loaded_config.as_ref(),
      &runtime_config,
      &full_address,
    )?;
    #[cfg(not(feature = "host_delegation_signing"))]
    let host_config_injection = None;
    let mech_paths = effective.paths;
    let stylesheet_paths = effective.stylesheet_paths;
    let wasm_pkg = effective.wasm_pkg.as_str();
    let shim_path = effective.shim_path.as_str();

    let wasm_path = format!("{wasm_pkg}/mech_wasm_bg.wasm.br");
    let js_path = format!("{wasm_pkg}/mech_wasm.js");

    println!("{badge} Loading resources…");

    print!("{badge} Loading stylesheet…");
    let stylesheet_str = load_stylesheets(&stylesheet_paths, &stylesheet_backup_url).await?;

    print!("{badge} Loading HTML shim…");
    let shim = read_or_download(
      shim_path,
      &shim_backup_url,
      Some(SHIMHTML.as_bytes()),
    )
    .await?;

    let shim_str = String::from_utf8(shim)
      .map_err(|e| {
        MechError::new(
          Utf8ConversionError {
            source_error: e.to_string(),
          },
          None,
        )
        .with_compiler_loc()
      })?;

    print!("{badge} Loading WASM…");
    let wasm = read_or_download(
      &wasm_path,
      &wasm_backup_url,
      Some(MECHWASM),
    )
    .await?;

    print!("{badge} Loading JS…");
    let js = read_or_download(
      &js_path,
      &js_backup_url,
      Some(MECHJS),
    )
    .await?;

    let authority = capabilities::build_mech_filesystem_authority(
      serve_matches,
      loaded_config.as_ref(),
      &badge,
    )?;

    let mut server = MechServer::new_with_runtime_config_and_host_config(
      "Mech Server".to_string(),
      full_address,
      stylesheet_str,
      shim_str,
      wasm,
      js,
      authority,
      runtime_config,
      host_config,
      host_config_injection,
      config_shim_at_root,
    );

    server.init().await?;

    if let Err(err) = server.load_workspace(&mech_paths) {
      println!("{error_badge} {err:#?}");
      std::process::exit(1);
    }

    println!("{badge} Sources loaded.");

    server.serve().await?;
  }

  // --------------------------------------------------------------------------
  // Test
  // --------------------------------------------------------------------------
  #[cfg(feature = "test")]
  if let Some(matches) = cli_matches.subcommand_matches("test") {
    let mech_paths: Vec<String> = matches
      .get_many::<String>("mech_test_file_paths")
      .map_or(vec![".".to_string()], |files| files.map(|file| file.to_string()).collect());
    let output_path = matches.get_one::<String>("output_path").cloned();
    let verbose = matches.get_flag("verbose");
    let exit_code = run_mech_tests(mech_paths, tree_flag, debug_flag, time_flag, trace_flag, output_path, verbose)?;
    std::process::exit(exit_code);
  }

  // --------------------------------------------------------------------------
  // Build
  // --------------------------------------------------------------------------
  #[cfg(feature = "build")]
  if let Some(matches) = cli_matches.subcommand_matches("build") {
    let mech_paths: Vec<String> = matches.get_many::<String>("mech_build_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let output_path = PathBuf::from(matches.get_one::<String>("output_path").cloned().unwrap_or(".".to_string()));
    let debug_flag = matches.get_flag("debug");
    let rounds_per_step = root_rounds_per_step.unwrap_or(10_000);
    // Create the directory html_output_path
    if output_path != PathBuf::from(".") {
      match fs::create_dir_all(&output_path) {
        Ok(_) => {
          println!("{} Directory created: {}", "[Created]".truecolor(153,221,85), output_path.display());
        }
        Err(err) => {
          println!("Error creating directory: {:?}", err);
        }
      }
    }

    let bytecode_count = validate_build_bytecode_inputs(&mech_paths)?;
    let bytecode = if bytecode_count == 1 {
      match mech_runtime::read_runtime_source_file(Path::new(&mech_paths[0]))? {
        MechSourceCode::ByteCode(bytecode) => bytecode,
        _ => unreachable!("bytecode input should load as MechSourceCode::ByteCode"),
      }
    } else {
      let uuid = generate_uuid();
      let mut program = MechProgram::new(MechProgramConfig { name: format!("program-{}", uuid), environment: MechProgramEnvironment::default() });
      let _ = tree_flag;
      let _ = trace_flag;
      program.configure(debug_flag, trace_flag, time_flag, rounds_per_step);
      for path in mech_paths {
        let source = mech_runtime::read_runtime_source_file(Path::new(&path))?;
        let _ = program.run_source(&source)?;
      }

      let bytecode = program.interpreter_mut().compile()?;

      // print debug info for the context
      if debug_flag {
        println!("{} Bytecode Size: {:#?} bytes", "[Debug]".truecolor(246,192,78), &program.interpreter().context);
      }

      bytecode
    };

    let output_file = output_path.join("output.mecb");

    let mut f = std::fs::File::create(&output_file)?;
    f.write_all(&bytecode)?;
    f.flush()?;

    println!("{} Mech bytecode written to: {}", "[Output]".truecolor(153,221,85), output_file.display());

    return Ok(());
  }

  // --------------------------------------------------------------------------
  // Format
  // --------------------------------------------------------------------------
  #[cfg(feature = "formatter")]
  if let Some(matches) = cli_matches.subcommand_matches("format") {
    let badge = "[Mech Formatter]".truecolor(34, 204, 187);
    let html_flag = matches.get_flag("html");
    let stylesheet_paths: Vec<String> = matches
        .get_many::<String>("stylesheet")
        .map_or(vec![], |paths| paths.map(|path| path.to_string()).collect());

    let shim_path = matches
        .get_one::<String>("shim")
        .cloned()
        .unwrap_or("".to_string());

    let output_arg = matches.get_one::<String>("output_path").cloned();
    let output_path = PathBuf::from(output_arg.clone().unwrap_or(".".to_string()));
    let is_output_file = output_path.extension().is_some();

    let mech_paths: Vec<String> = matches
        .get_many::<String>("mech_format_file_paths")
        .map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let output_matches_input_dir = format_output_matches_input_dir(&mech_paths, &output_path, is_output_file)?;
    reject_ambiguous_matching_output_dir(output_matches_input_dir, mech_paths.len(), &output_path)?;
    let writes_in_place = format_writes_in_place(output_arg.as_deref(), &output_path, is_output_file)? || output_matches_input_dir;

    // If the user provided exactly one path
    if mech_paths.len() == 1 {
      let input_path = PathBuf::from(&mech_paths[0]);
      if input_path.is_dir() && is_output_file {
        return Err(MechError::new(
          GenericError {
            msg: format!(
              "Cannot write directory `{}` into single output file `{}`. Provide a directory for --out instead.",
              input_path.display(),
              output_path.display(),
            ),
          },
          None,
        ).with_compiler_loc());
      }
    }
    println!("{} Loading resources…", badge);

    // Load stylesheet
    print!("{} Loading stylesheet…", badge);
    let stylesheet_str = load_stylesheets(&stylesheet_paths, &stylesheet_backup_url).await?;

    // Load shim HTML
    print!("{} Loading HTML shim…", badge);
    let shim = read_or_download(&shim_path, &shim_backup_url, Some(SHIMHTML.as_bytes())).await?;
    let shim_str = String::from_utf8(shim).map_err(|e| {
      MechError::new(
        Utf8ConversionError {
          source_error: e.to_string(),
        },
        None,
      )
      .with_compiler_loc()
    })?;

    let output_exclusion = if writes_in_place {
      None
    } else {
      format_output_exclusion(output_arg.as_deref(), &output_path, is_output_file)?
    };
    let mut loaded_sources: Vec<(CollectedSourceTarget, MechSourceCode)> = Vec::new();
    for path in mech_paths {
      for target in collect_format_targets(Path::new(&path), output_exclusion.as_deref(), html_flag, writes_in_place)? {
        let code = read_format_source(&target.path)?;
        loaded_sources.push((target, code));
      }
    }
    reject_multi_target_file_output(loaded_sources.len(), &output_path, is_output_file)?;
    let format_targets: Vec<CollectedSourceTarget> = loaded_sources.iter().map(|(target, _)| target.clone()).collect();
    ensure_unique_format_outputs(&format_targets, &output_path, is_output_file, writes_in_place, html_flag)?;

    // Only create directory if output_path is not a file
    if !is_output_file && output_path != PathBuf::from(".") {
      match fs::create_dir_all(&output_path) {
        Ok(_) => println!(
          "{} Directory created: {}",
          "[Created]".truecolor(153, 221, 85),
          output_path.display()
        ),
        Err(err) => println!("Error creating directory: {:?}", err),
      }
    }

    // HTML mode
    if html_flag {
      let mut html_items: Vec<(CollectedSourceTarget, String)> = Vec::new();
      for (target, src) in &loaded_sources {
        let html = match src {
          MechSourceCode::Html(content) => content.clone(),
          MechSourceCode::String(source) => {
            let tree = parser::parse(source.trim())?;
            let mut formatter = Formatter::new();
            formatter.format_html(&tree, stylesheet_str.clone(), shim_str.clone())
          }
          other => return Err(MechError::new(GenericError { msg: format!("Unsupported source kind for HTML formatting `{}`: {:?}", target.path.display(), other) }, None).with_compiler_loc()),
        };
        html_items.push((target.clone(), html));
      }
      if is_output_file && html_items.len() == 1 {
        let (_, content) = html_items.remove(0);
        save_to_file(output_path, &content)?;
      } else {
        for (target, content) in html_items {
          let output_file = format_output_file_for_target(&target, &output_path, is_output_file, writes_in_place, true);
          save_to_file(output_file, &content)?;
        }
      }
    } else {
      // Raw source mode
      for (target, mech_src) in loaded_sources {
        let content = match mech_src {
          MechSourceCode::String(source) => {
            let tree = parser::parse(source.trim())?;
            let mut formatter = Formatter::new();
            formatter.format(&tree)
          }
          MechSourceCode::Html(content) => content,
          other => return Err(MechError::new(GenericError { msg: format!("Unsupported source kind for raw formatting `{}`: {:?}", target.path.display(), other) }, None).with_compiler_loc()),
        };
        let output_file = format_output_file_for_target(&target, &output_path, is_output_file, writes_in_place, false);
        save_to_file(output_file, &content)?;
      }
    }

    return Ok(());
  }


  // --------------------------------------------------------------------------
  // Run
  // --------------------------------------------------------------------------
  let mut caught_inturrupts = Arc::new(Mutex::new(0));
  #[cfg(feature = "run")]
  let uuid = generate_uuid();
  #[cfg(feature = "run")]
  let mut repl_runtime_config: Option<RuntimeConfig> = None;
  #[cfg(all(feature = "run", feature = "repl"))]
  let mut repl_seed_program: Option<MechProgram> = None;

  #[cfg(feature = "run")]
  {
    let run_matches = cli_matches.subcommand_matches("run");
    let explicit_run_command = run_matches.is_some();
    let mut run_inputs: Vec<String> = if let Some(run_matches) = run_matches {
      run_matches
        .get_many::<String>("mech_run_paths")
        .map_or(vec![], |files| files.map(|file| file.to_string()).collect())
    } else if let Some(m) = cli_matches.get_many::<String>("mech_paths") {
      m.map(|s| s.to_string()).collect()
    } else {
      vec![]
    };
    run_inputs.extend(cli_host_capability_passthrough_values(&cli_matches, run_matches));

    let run_debug_flag = debug_flag || run_matches.map(|m| m.get_flag("debug")).unwrap_or(false);
    let run_trace_flag = trace_flag || run_matches.map(|m| m.get_flag("trace")).unwrap_or(false);
    let run_time_flag = time_flag || run_matches.map(|m| m.get_flag("time")).unwrap_or(false);
    let run_rounds_per_step = run_matches
      .and_then(|m| m.get_one::<String>("rounds-per-step"))
      .and_then(|s| s.parse::<usize>().ok())
      .or(root_rounds_per_step);

    let run_input_mode = classify_run_inputs(run_inputs);
    let config_matches = run_matches.unwrap_or(&cli_matches);
    let config_inputs: Vec<String> = match &run_input_mode {
      RunInputMode::Paths(paths) => paths.clone(),
      RunInputMode::Empty | RunInputMode::InlineSource(_) => Vec::new(),
    };
    let loaded_config = config::load_run_cli_config(config_matches, &config_inputs)?;

    let runtime_config = effective_run_runtime_config(
      loaded_config.as_ref(),
      format!("program-{}", uuid),
      run_debug_flag,
      run_trace_flag,
      run_time_flag,
      run_rounds_per_step,
    )?;
    repl_runtime_config = Some(runtime_config.clone());

    let cli_capability_selection = cli_host_capability_selection(&cli_matches, run_matches);
    let cli_grants = config::effective_cli_host_grants(
      loaded_config.as_ref(),
      cli_capability_selection,
    )?;

    let configured_hosts = loaded_config
      .as_ref()
      .map(|loaded| loaded.document.hosts.as_slice())
      .unwrap_or(&[]);

    let configured_run_grants = loaded_config
      .as_ref()
      .and_then(|loaded| loaded.document.run.as_ref())
      .map(|run| run.grants.as_slice())
      .unwrap_or(&[]);

    let badge = "[Mech Run]".truecolor(34, 204, 187);
    let mut fs_authority = capabilities::build_mech_filesystem_authority(
      config_matches,
      loaded_config.as_ref(),
      &badge,
    )?;

    let mut runtime = new_cli_runtime(runtime_config, &cli_grants, configured_hosts, configured_run_grants)?;
    let fs_kernel = fs_authority.kernel().clone();
    runtime.set_source_resolver(FileSourceResolver::new(&std::env::current_dir()?).with_capabilities(fs_kernel.clone(), MECH_TOOL_SUBJECT));

    if let RunInputMode::InlineSource(source) = &run_input_mode {
      match run_cli_source(&mut runtime, source.trim()) {
        Ok(r) => {
          println!("{}", r.kind());
          #[cfg(feature = "pretty_print")]
          println!("{}", r.pretty_print());
          #[cfg(not(feature = "pretty_print"))]
          println!("{:#?}", r);
          std::process::exit(0);
        }
        Err(err) => {
          println!("{} {:#?}",
            "[Error]".truecolor(246,98,78),
            err
          );
          std::process::exit(1);
        }
      }
    }

    let run_paths = match run_input_mode {
      RunInputMode::Paths(paths) => paths,
      RunInputMode::Empty => Vec::new(),
      RunInputMode::InlineSource(_) => unreachable!("inline source exits before path execution"),
    };

    let options = config::effective_run_options(
      run_paths,
      loaded_config.as_ref(),
      explicit_run_command,
    )?;

    let result: MResult<Value> = if let Some(options) = options {
      if !config_matches.get_flag("no_default_capabilities") {
        let mut ids = mech_runtime::DefaultIdGenerator::new();
        for p in &options.paths {
          let path = Path::new(p);
          let grant_path = if path.is_dir() { path } else { path.parent().unwrap_or_else(|| Path::new(".")) };
          fs_authority.grant_path(&mut ids, grant_path, true, [FS_READ, FS_LIST, mech_runtime::FS_RESOLVE, mech_runtime::FS_IMPORT])?;
        }
      }
      let fs_kernel = fs_authority.kernel().clone();
      runtime.set_source_resolver(FileSourceResolver::new(&std::env::current_dir()?).with_capabilities(fs_kernel.clone(), MECH_TOOL_SUBJECT));
      let mut last = Value::Empty;
      for p in &options.paths {
        for target in collect_run_targets_with_capabilities(Path::new(p), &fs_kernel)? {
          let src = mech_runtime::read_runtime_source_file_with_capabilities(&target, Some(&fs_kernel), Some(MECH_TOOL_SUBJECT))?;
          last = run_cli_source_code(&mut runtime, &src)?;
        }
      }
      Ok(last)
    } else {
      repl_flag = true;
      Ok(Value::Empty)
    };

    match &result {
      Ok(r) if repl_flag => {
        #[cfg(feature = "repl")]
        {
          repl_seed_program = Some(runtime.take_program());
        }
        #[cfg(not(feature = "repl"))]
        {
          println!("{}", r.kind());
          #[cfg(feature = "pretty_print")]
          println!("{}", r.pretty_print());
          #[cfg(not(feature = "pretty_print"))]
          println!("{:#?}", r);
          std::process::exit(0);
        }
      }
      Ok(r) => {
        println!("{}", r.kind());
        #[cfg(feature = "pretty_print")]
        println!("{}", r.pretty_print());
        #[cfg(not(feature = "pretty_print"))]
        println!("{:#?}", r);
        std::process::exit(0);
      }
      Err(err) => {
        print_mech_error(err);
        std::process::exit(1);
      }
    }

    #[cfg(windows)]
    control::set_virtual_terminal(true).unwrap();
    clc();
    let mut stdo = stdout();
    stdo.execute(Print(text_logo));
    stdo.execute(cursor::MoveToNextLine(1));
    println!("\n                {}                ",format!("v{}",VERSION).truecolor(246,192,78));
    println!("           {}           \n", "www.mech-lang.org");
    let intro_message = format!("{}Enter {} for a list of all commands.{}\n", mika_open, help_cmd, mika_close);
    println!("{} {}", micromika, intro_message);

    // Catch Ctrl-C a couple times before quitting
    let mut ci = caught_inturrupts.clone();
    ctrlc::set_handler(move || {
      println!("{}", ctrlc_cmd);
      let mut caught_inturrupts = ci.lock().unwrap();
      *caught_inturrupts += 1;
      if *caught_inturrupts >= 3 {
        let final_state = ProgressBar::new_spinner();
        let completed_style = ProgressStyle::with_template(
          "\n{spinner:.yellow} {msg}"
        ).unwrap().tick_strings(MICROMIKA_WAVE);
        final_state.set_style(completed_style);
        final_state.set_message(format!("{}Okay cya!{}\n", mika_open, mika_close));
        for _ in 0..MICROMIKA_WAVE.len() - 1 {
          thread::sleep(Duration::from_millis(100));
          final_state.tick();
        }
        std::process::exit(0);
      }
      println!("\n{} {}Enter {} to terminate this REPL session.{}\n", micromika_point, mika_open, quit_cmd, mika_close);
      print_prompt();
    }).expect("Error setting Ctrl+C handler");
  }

  // --------------------------------------------------------------------------
  // REPL
  // --------------------------------------------------------------------------
  #[cfg(all(feature = "repl", feature = "run"))]
  // TODO: move the REPL onto MechRuntime as a separate PR so CLI host contexts work interactively too.
  let mut repl = {
    if let Some(program) = repl_seed_program {
      MechRepl::from(program)
    } else {
      let config = repl_runtime_config.unwrap_or_else(RuntimeConfig::default);
      let mut repl_program = MechProgram::new(MechProgramConfig {
        name: config.name.clone(),
        environment: MechProgramEnvironment::default(),
      });
      repl_program.configure(
        config.diagnostics.debug_enabled,
        config.diagnostics.trace_enabled,
        config.diagnostics.profile_enabled,
        config.limits.max_steps_per_turn.unwrap_or(10_000) as usize,
      );
      MechRepl::from(repl_program)
    }
  };
  #[cfg(all(feature = "repl", not(feature = "run")))]
  let mut repl = MechRepl::from(MechProgram::new(MechProgramConfig {
    name: format!("repl-{}", generate_uuid()),
    environment: MechProgramEnvironment::default(),
  }));
  #[cfg(feature = "repl")]
  'REPL: loop {
    {
      let mut ci = caught_inturrupts.lock().unwrap();
      *ci = 0;
    }
    // Prompt the user for input
    print_prompt();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    // Parse the input
    if input.chars().nth(0) == Some(':') {
      match parse_repl_command(&input.as_str()) {
        Ok((_, repl_command)) => {
          match repl.execute_repl_command(repl_command) {
            Ok(output) => {
              println!("{}", output);
            }
            Err(err) => {
              println!("!{:?}", err);
            }
          }
        }
        Err(x) => {
          println!("{} Unrecognized command: {}", "[Error]".truecolor(246,98,78), x);
        }
      }
    } else if input.trim() == "" {
      continue;
    } else {
      let cmd = ReplCommand::Code(vec![("repl".to_string(),MechSourceCode::String(input))]);
      match repl.execute_repl_command(cmd) {
        Ok(output) => {
          println!("{}", output);
        }
        Err(err) => {
          println!("(x)> {:#?}", err);
        }
      }
    }
  }

  Ok(())
}

#[cfg(feature = "async")]
pub async fn load_resource(resource_path: &str) -> String {
  if resource_path.starts_with("http") {
    match reqwest::get(resource_path).await {
      Ok(response) => match response.text().await {
        Ok(text) => text,
        Err(err) => {
          eprintln!("Error fetching resource text: {:?}", err);
          String::new()
        }
      },
      Err(err) => {
        eprintln!("Error fetching resource: {:?}", err);
        String::new()
      }
    }
  } else {
    match tokio::fs::read_to_string(resource_path).await {
      Ok(content) => content,
      Err(err) => {
        eprintln!("Error reading resource file: {:?}", err);
        String::new()
      }
    }
  }
}

#[cfg(not(feature = "async"))]
pub fn load_resource(resource_path: &str) -> String {
  if resource_path.starts_with("http") {
    match reqwest::blocking::get(resource_path) {
      Ok(response) => match response.text() {
        Ok(text) => text,
        Err(err) => {
          eprintln!("Error fetching resource text: {:?}", err);
          String::new()
        }
      },
      Err(err) => {
        eprintln!("Error fetching resource: {:?}", err);
        String::new()
      }
    }
  } else {
    match std::fs::read_to_string(resource_path) {
      Ok(content) => content,
      Err(err) => {
        eprintln!("Error reading resource file: {:?}", err);
        String::new()
      }
    }
  }
}


#[cfg(all(feature = "host_delegation_signing", feature = "serve"))]
fn host_delegation_args() -> Vec<Arg> {
  vec![
    Arg::new("host_delegation_key").long("host-delegation-key").value_name("PATH").num_args(1),
    Arg::new("host_delegation_public_key").long("host-delegation-public-key").value_name("PATH").num_args(1),
    Arg::new("host_delegation_key_id").long("host-delegation-key-id").value_name("ID").num_args(1),
    Arg::new("host_delegation_issuer").long("host-delegation-issuer").value_name("ISSUER").num_args(1),
    Arg::new("host_delegation_subject").long("host-delegation-subject").value_name("SUBJECT").num_args(1),
    Arg::new("host_delegation_audience").long("host-delegation-audience").value_name("AUDIENCE").num_args(1),
    Arg::new("host_delegation_expires_ms").long("host-delegation-expires-ms").value_name("MS").num_args(1),
  ]
}

#[cfg(any(not(feature = "host_delegation_signing"), not(feature = "serve")))]
fn host_delegation_args() -> Vec<Arg> {
  Vec::new()
}

#[cfg(all(feature = "host_delegation_signing", feature = "serve"))]
fn serve_host_delegation_injection(
  matches: &clap::ArgMatches,
  loaded_config: Option<&mech::LoadedMechConfig>,
  runtime_config: &mech_runtime::RuntimeConfig,
  full_address: &str,
) -> MResult<Option<mech::HostAuthorityInjection>> {
  let Some(private_key) = matches.get_one::<String>("host_delegation_key") else {
    return Ok(None);
  };
  let public_key = matches
    .get_one::<String>("host_delegation_public_key")
    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "--host-delegation-public-key is required with --host-delegation-key"))?;
  let Some(loaded_config) = loaded_config else {
    return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "host delegation signing requires a loaded config").into());
  };
  let current_dir = std::env::current_dir()?;
  let options = mech::HostDelegationSigningOptions {
    private_key_path: current_dir.join(private_key),
    public_key_path: current_dir.join(public_key),
    key_id: matches.get_one::<String>("host_delegation_key_id").cloned().unwrap_or_else(|| "dev".to_string()),
    issuer: matches.get_one::<String>("host_delegation_issuer").cloned().unwrap_or_else(|| "host://mech-cli".to_string()),
    subject: matches.get_one::<String>("host_delegation_subject").cloned().unwrap_or_else(|| "wasm://browser".to_string()),
    audience: matches.get_one::<String>("host_delegation_audience").cloned().unwrap_or_else(|| format!("browser://serve/{full_address}")),
    expires_ms: matches.get_one::<String>("host_delegation_expires_ms").map(|value| value.parse()).transpose().map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "--host-delegation-expires-ms must be an integer"))?,
  };
  let host_config = mech::web_runtime_injection_config_from_document(
    &loaded_config.document,
    runtime_config,
  )?;
  let now_ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))?
    .as_millis() as u64;
  mech::signed_browser_runtime_injection_config(host_config, &options, now_ms).map(Some)
}

fn source_range_to_offset_range(file_content: &str, range: &SourceRange) -> (usize, usize) {
  let mut offset = 0;
  let mut start_offset = 0;
  let mut end_offset = 0;

  for (line_index, line) in file_content.split_inclusive('\n').enumerate() {
    let row = line_index + 1;
    let line_len = line.len();
    if row == range.start.row {
      start_offset = offset + (range.start.col - 1);
    }
    if row == range.end.row {
      end_offset = offset + (range.end.col - 1);
      break;
    }
    offset += line_len;
  }
  end_offset = end_offset.min(file_content.len());
  while start_offset < end_offset
    && file_content.as_bytes()[start_offset].is_ascii_whitespace()
  {
    start_offset += 1;
  }
  while end_offset > start_offset
    && file_content.as_bytes()[end_offset - 1].is_ascii_whitespace()
  {
    end_offset -= 1;
  }
  if end_offset <= start_offset {
    end_offset = start_offset + 1;
    // Clamp in case we were at EOF
    end_offset = end_offset.min(file_content.len());
  }
  (start_offset, end_offset)
}

pub fn print_mech_error(err: &MechError) {
  if let Some(watch_error) = err.kind_as::<WatchPathFailed>() {
    let src_file_path = watch_error.file_path.to_string();
    match &err.source {
      Some(src_err) => {
        if let Some(report) = &src_err.kind_as::<ParserErrorReport>() {
          let first_error_range = report.1.first().map(|e| e.cause_rng.clone()).unwrap_or(SourceRange::default());
          let (first_start, first_end) = source_range_to_offset_range(&report.0, &first_error_range);
          let mut error_report = Report::build(ReportKind::Error, (src_file_path.clone(), first_start..first_end))
              .with_message(format!("Syntax Errors Found: {}", report.1.len()));

          for (err_num, err_ctx) in report.1.iter().enumerate() {
            let (start, end) = source_range_to_offset_range(&report.0, &err_ctx.cause_rng);

            if let Some(annotation_rng) = err_ctx.annotation_rngs.first() {
              let (ann_start, ann_end) = source_range_to_offset_range(&report.0, annotation_rng);

              error_report = error_report.with_label(
                Label::new((src_file_path.clone(), ann_start..ann_end))
                      .with_message(format!(
                        "#{}: {} [{}:{}]",
                        err_num + 1,
                        err_ctx.err_message,
                        annotation_rng.start.row,
                        annotation_rng.start.col
                      ))
                    .with_color(Color::Yellow),
              );
            } else {
              error_report = error_report.with_label(
                Label::new((src_file_path.clone(), start..end))
                      .with_message(format!(
                        "#{}: {} [{}:{}]",
                        err_num + 1,
                        err_ctx.err_message,
                        err_ctx.cause_rng.start.row,
                        err_ctx.cause_rng.start.col
                      ))
                    .with_color(Color::Yellow),
              );
            }
          }
          let cache = sources([(src_file_path.clone(), report.0.clone())]);
          error_report.finish().print(cache).unwrap_or_else(|e| {
            println!("Error printing report: {:?}", e);
          });
        } else {
          println!("Error:");
          println!("{:#?}", err);
        }
      }
      None => {
        println!("Error:");
        println!("{:#?}", err);
      }
    }
  } else {
      println!("Error:");
      println!("{:#?}", err);
  }
}

#[cfg(all(test, feature = "serve"))]
mod filesystem_capability_cli_tests {
    use super::*;
    use mech_runtime::{DefaultIdGenerator, SERVE_HOST_SUBJECT, FS_IMPORT, FS_LIST, FS_READ, FS_RESOLVE, FS_SERVE, FS_WATCH};

    fn capability_matches(arguments: &[&str]) -> clap::ArgMatches {
        capabilities::add_filesystem_capability_args(Command::new("mech").subcommand(
            Command::new("serve").arg(Arg::new("mech_serve_file_paths").action(ArgAction::Append)),
        ))
        .try_get_matches_from(arguments)
        .unwrap()
        .subcommand_matches("serve")
        .unwrap()
        .clone()
    }

    fn temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "mech-cli-capability-{label}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir_all(&root).unwrap();
        root.canonicalize().unwrap()
    }

    fn test_badge() -> ColoredString {
        "[Mech Server]".normal()
    }

    #[test]
    fn default_grants_current_directory_when_no_capability_options_are_present() {
        let matches = capability_matches(&["mech", "serve", "."]);
        let authority = capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
        let mut ids = DefaultIdGenerator::new();
        authority
            .delegate_path_to(
                &mut ids,
                SERVE_HOST_SUBJECT,
                &std::env::current_dir().unwrap(),
                true,
                [FS_READ, FS_LIST, FS_WATCH, FS_RESOLVE, FS_IMPORT, FS_SERVE],
            )
            .unwrap();
    }

    #[test]
    fn cap_root_disables_default_current_directory_authority() {
        let root = temp_root("cap-root");
        let allowed = root.join("allowed");
        let outside = root.join("outside");
        std::fs::create_dir_all(&allowed).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let allowed_arg = allowed.to_string_lossy();
        let outside_arg = outside.to_string_lossy();
        let matches =
            capability_matches(&["mech", "--cap-root", &allowed_arg, "serve", &outside_arg]);
        let authority = capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
        let mut ids = DefaultIdGenerator::new();
        assert!(
            authority
                .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &outside, true, [FS_READ])
                .is_err()
        );
        authority
            .delegate_path_to(
                &mut ids,
                SERVE_HOST_SUBJECT,
                &allowed,
                true,
                [FS_READ, FS_SERVE],
            )
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_default_capabilities_grants_nothing() {
        let root = temp_root("none");
        let matches = capability_matches(&[
            "mech",
            "--no-default-capabilities",
            "serve",
            root.to_str().unwrap(),
        ]);
        let authority = capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
        let mut ids = DefaultIdGenerator::new();
        assert!(
            authority
                .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &root, true, [FS_READ])
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn allow_read_does_not_grant_serve() {
        let root = temp_root("read-only");
        let matches = capability_matches(&[
            "mech",
            "serve",
            root.to_str().unwrap(),
            "--allow-read",
            root.to_str().unwrap(),
        ]);
        let authority = capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
        let mut ids = DefaultIdGenerator::new();
        authority
            .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &root, true, [FS_READ])
            .unwrap();
        assert!(
            authority
                .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &root, true, [FS_SERVE])
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_granular_grants_combine_for_normal_serve_directory() {
        let root = temp_root("granular-combine");
        let allowed = root.join("allowed");
        std::fs::create_dir_all(&allowed).unwrap();
        let allowed_arg = allowed.to_string_lossy();
        let matches = capability_matches(&[
            "mech",
            "--allow-read",
            &allowed_arg,
            "--allow-watch",
            &allowed_arg,
            "--allow-serve",
            &allowed_arg,
            "serve",
            &allowed_arg,
        ]);
        let authority = capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
        assert_eq!(authority.source_capabilities().len(), 1);
        let mut ids = DefaultIdGenerator::new();
        authority
            .delegate_path_to(
                &mut ids,
                SERVE_HOST_SUBJECT,
                &allowed,
                true,
                [FS_READ, FS_LIST, FS_WATCH, FS_RESOLVE, FS_IMPORT, FS_SERVE],
            )
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn allow_serve_grants_serve() {
        let root = temp_root("serve-only");
        let matches = capability_matches(&[
            "mech",
            "serve",
            root.to_str().unwrap(),
            "--allow-serve",
            root.to_str().unwrap(),
        ]);
        let authority = capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
        let mut ids = DefaultIdGenerator::new();
        authority
            .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &root, true, [FS_SERVE])
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(all(test, feature = "build"))]
mod build_input_tests {
  use super::*;

  fn paths(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
  }

  #[test]
  fn build_rejects_mixed_source_then_bytecode() {
    let error = validate_build_bytecode_inputs(&paths(&["old.mec", "compiled.mecb"]))
      .unwrap_err()
      .full_chain_message();
    assert!(error.contains("Cannot mix bytecode"));
  }

  #[test]
  fn build_rejects_bytecode_then_source() {
    let error = validate_build_bytecode_inputs(&paths(&["compiled.mecb", "next.mec"]))
      .unwrap_err()
      .full_chain_message();
    assert!(error.contains("Cannot mix bytecode"));
  }

  #[test]
  fn build_rejects_multiple_bytecode_inputs() {
    let error = validate_build_bytecode_inputs(&paths(&["a.mecb", "b.mecb"]))
      .unwrap_err()
      .full_chain_message();
    assert!(error.contains("Cannot combine multiple bytecode"));
  }

  #[test]
  fn build_single_bytecode_input_is_allowed_for_clean_copy() {
    assert_eq!(validate_build_bytecode_inputs(&paths(&["compiled.mecb"])).unwrap(), 1);
  }

  #[test]
  fn build_multiple_source_inputs_still_work() {
    assert_eq!(validate_build_bytecode_inputs(&paths(&["a.mec", "b.mec"])).unwrap(), 0);
  }
}

#[cfg(all(test, feature = "formatter"))]
mod format_collection_tests {
  use super::*;

  fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
      "mech-format-collection-{label}-{}",
      std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
  }

  fn format_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap()
  }

  #[test]
  fn collect_format_targets_preserves_duplicate_basenames_under_directory() {
    let _guard = format_test_lock();
    let root = temp_root("duplicates");
    let docs = root.join("docs");
    std::fs::create_dir_all(docs.join("a")).unwrap();
    std::fs::create_dir_all(docs.join("b")).unwrap();
    std::fs::write(docs.join("a/index.mec"), "x := 1").unwrap();
    std::fs::write(docs.join("b/index.mec"), "x := 2").unwrap();

    let targets = collect_format_targets(&docs, None, false, false).unwrap();
    let relatives = targets.iter().map(|target| target.relative_path.clone()).collect::<Vec<_>>();
    assert_eq!(relatives, vec![PathBuf::from("a/index.mec"), PathBuf::from("b/index.mec")]);
    assert!(targets.iter().all(|target| target.input_root == docs));
    ensure_unique_format_outputs(&targets, &root.join("out"), false, false, true).unwrap();
    ensure_unique_format_outputs(&targets, &root.join("out"), false, false, false).unwrap();
    assert_eq!(root.join("out").join(&targets[0].relative_path).with_extension("html"), root.join("out/a/index.html"));
    assert_eq!(root.join("out").join(&targets[1].relative_path), root.join("out/b/index.mec"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_directory_output_path_selection_preserves_default_and_output_roots() {
    let _guard = format_test_lock();
    let root = temp_root("default-output-paths");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("docs/main.mec"), "x := 1").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let targets = collect_format_targets(Path::new("docs"), None, false, false).unwrap();
    let target = &targets[0];
    assert!(format_writes_in_place(None, Path::new("."), false).unwrap());
    assert!(format_writes_in_place(Some("."), Path::new("."), false).unwrap());
    assert!(format_writes_in_place(Some("./"), Path::new("./"), false).unwrap());
    assert!(format_writes_in_place(Some(root.to_str().unwrap()), &root, false).unwrap());
    assert!(!format_writes_in_place(Some("out"), Path::new("out"), false).unwrap());
    std::env::set_current_dir(old_cwd).unwrap();

    assert_eq!(format_output_file_for_target(target, Path::new("."), false, true, false), PathBuf::from("docs/main.mec"));
    assert_eq!(format_output_file_for_target(target, Path::new("."), false, true, true), PathBuf::from("docs/main.html"));
    assert_eq!(format_output_file_for_target(target, Path::new("."), false, true, false), PathBuf::from("docs/main.mec"));
    assert_eq!(format_output_file_for_target(target, Path::new("out"), false, false, false), PathBuf::from("out/main.mec"));
    assert_eq!(format_output_file_for_target(target, Path::new("out"), false, false, true), PathBuf::from("out/main.html"));
    assert_eq!(format_output_file_for_target(target, Path::new("single.mec"), true, false, false), PathBuf::from("single.mec"));
    assert_eq!(format_output_file_for_target(target, Path::new("page.htm"), true, false, true), PathBuf::from("page.htm"));
    assert_eq!(format_output_file_for_target(target, Path::new("page.html"), true, false, true), PathBuf::from("page.html"));
    assert_eq!(format_output_file_for_target(target, Path::new("page.custom"), true, false, true), PathBuf::from("page.custom"));
    std::fs::remove_dir_all(root).unwrap();
  }


  #[test]
  fn collect_format_targets_preserves_explicit_relative_file_path() {
    let _guard = format_test_lock();
    let root = temp_root("explicit-relative");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.mec"), "x := 1").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let targets = collect_format_targets(Path::new("src/main.mec"), None, false, false).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    assert_eq!(targets[0].relative_path, PathBuf::from("src/main.mec"));
    assert_eq!(targets[0].default_output_path, PathBuf::from("src/main.mec"));
    assert_eq!(format_output_file_for_target(&targets[0], Path::new("."), false, true, false), PathBuf::from("src/main.mec"));
    assert_eq!(root.join("out").join(&targets[0].relative_path), root.join("out/src/main.mec"));
    assert_eq!(root.join("out").join(&targets[0].relative_path).with_extension("html"), root.join("out/src/main.html"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn safe_output_relative_path_rejects_parent_components() {
    let _guard = format_test_lock();
    assert_eq!(safe_output_relative_path(Path::new("../docs/main.mec")).unwrap(), PathBuf::from("main.mec"));
  }

  #[test]
  fn format_explicit_parent_relative_input_under_out_uses_filename() {
    let _guard = format_test_lock();
    let root = temp_root("parent-relative-out");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::create_dir_all(root.join("examples")).unwrap();
    std::fs::write(root.join("docs/main.mec"), "x := 1").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(root.join("examples")).unwrap();
    let targets = collect_format_targets(Path::new("../docs/main.mec"), None, false, false).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    let raw_output = format_output_file_for_target(&targets[0], Path::new("formatted"), false, false, false);
    let html_output = format_output_file_for_target(&targets[0], Path::new("formatted"), false, false, true);
    assert_eq!(raw_output, PathBuf::from("formatted/main.mec"));
    assert_eq!(html_output, PathBuf::from("formatted/main.html"));
    assert_eq!(format_output_file_for_target(&targets[0], Path::new("."), false, true, false), PathBuf::from("../docs/main.mec"));
    assert!(!raw_output.components().any(|component| matches!(component, std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_))));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_explicit_safe_relative_input_under_out_preserves_subdir() {
    let _guard = format_test_lock();
    let root = temp_root("safe-relative-out");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.mec"), "x := 1").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let targets = collect_format_targets(Path::new("src/main.mec"), None, false, false).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    let output = format_output_file_for_target(&targets[0], Path::new("formatted"), false, false, false);
    assert_eq!(output, PathBuf::from("formatted/src/main.mec"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_output_collision_uses_actual_output_paths() {
    let _guard = format_test_lock();
    let root = temp_root("actual-collisions");
    std::fs::create_dir_all(root.join("a")).unwrap();
    std::fs::create_dir_all(root.join("b")).unwrap();
    std::fs::write(root.join("a/main.mec"), "x := 1").unwrap();
    std::fs::write(root.join("b/main.mec"), "y := 2").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let mut targets = Vec::new();
    targets.extend(collect_format_targets(Path::new("a"), None, false, false).unwrap());
    targets.extend(collect_format_targets(Path::new("b"), None, false, false).unwrap());
    std::env::set_current_dir(old_cwd).unwrap();

    ensure_unique_format_outputs(&targets, Path::new("."), false, true, false).unwrap();
    let error = format!("{:?}", ensure_unique_format_outputs(&targets, Path::new("out"), false, false, false).unwrap_err());
    assert!(error.contains("Format output collision"), "got {error}");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn raw_format_preserves_include_directives_without_expanding() {
    let _guard = format_test_lock();
    let root = temp_root("include");
    std::fs::write(root.join("included.mec"), "secret := 42\n").unwrap();
    std::fs::write(root.join("main.mec"), "+> ./included.mec\nvisible := 1\n").unwrap();
    let source = read_format_source(&root.join("main.mec")).unwrap();
    let formatted = match source {
      MechSourceCode::String(text) => {
        let tree = parser::parse(text.trim()).unwrap();
        let mut formatter = Formatter::new();
        formatter.format(&tree)
      }
      other => panic!("expected string source, got {other:?}"),
    };
    assert!(formatted.contains("+> ./included.mec"), "formatted output was {formatted}");
    assert!(formatted.contains("visible"), "formatted output was {formatted}");
    assert!(!formatted.contains("secret := 42"), "formatted output was {formatted}");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn collectors_skip_symlinked_directory_loops() {
    let _guard = format_test_lock();
    use std::os::unix::fs::symlink;
    let root = temp_root("symlink-loop");
    std::fs::write(root.join("main.mec"), "x := 1").unwrap();
    symlink(&root, root.join("self")).unwrap();

    let format_targets = collect_format_targets(&root, None, false, false).unwrap();
    assert_eq!(format_targets.len(), 1);
    assert_eq!(format_targets[0].relative_path, PathBuf::from("main.mec"));

    let run_targets = collect_run_targets(&root).unwrap();
    assert_eq!(run_targets, vec![root.join("main.mec")]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn collect_format_targets_includes_symlinked_files_but_not_dirs_or_broken_links() {
    let _guard = format_test_lock();
    use std::os::unix::fs::symlink;
    let root = temp_root("format-symlink-file");
    std::fs::write(root.join("main.mec"), "x := 1").unwrap();
    symlink(root.join("main.mec"), root.join("linked.mec")).unwrap();
    symlink(&root, root.join("self")).unwrap();
    symlink(root.join("missing.mec"), root.join("broken.mec")).unwrap();

    let targets = collect_format_targets(&root, None, false, false).unwrap();
    let relatives = targets.iter().map(|target| target.relative_path.clone()).collect::<Vec<_>>();
    assert_eq!(relatives, vec![PathBuf::from("linked.mec"), PathBuf::from("main.mec")]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_default_output_has_no_exclusion() {
    let _guard = format_test_lock();
    let exclusion = format_output_exclusion(None, Path::new("."), false).unwrap();
    assert!(exclusion.is_none());
  }

  #[test]
  fn format_explicit_dot_output_has_no_exclusion() {
    let _guard = format_test_lock();
    let exclusion = format_output_exclusion(Some("."), Path::new("."), false).unwrap();
    assert!(exclusion.is_none());
  }

  #[test]
  fn format_docs_default_and_dot_outputs_collect_docs_sources() {
    let _guard = format_test_lock();
    let root = temp_root("default-output");
    let docs = root.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("main.mec"), "x := 1").unwrap();
    std::fs::write(docs.join("other.mec"), "y := 2").unwrap();

    let default_targets = collect_format_targets(&docs, format_output_exclusion(None, Path::new("."), false).unwrap().as_deref(), false, false).unwrap();
    assert_eq!(default_targets.len(), 2);

    let dot_targets = collect_format_targets(&docs, format_output_exclusion(Some("."), Path::new("."), false).unwrap().as_deref(), false, false).unwrap();
    assert_eq!(dot_targets.len(), 2);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_html_directory_in_place_skips_generated_html_siblings() {
    let _guard = format_test_lock();
    let root = temp_root("html-in-place-skip");
    let docs = root.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("foo.mec"), "x := 1").unwrap();
    std::fs::write(docs.join("foo.html"), "<html></html>").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let targets = collect_format_targets(Path::new("docs"), None, true, true).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].path, PathBuf::from("docs/foo.mec"));
    ensure_unique_format_outputs(&targets, Path::new("."), false, true, true).unwrap();
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_html_explicit_html_file_still_collected() {
    let _guard = format_test_lock();
    let root = temp_root("explicit-html");
    let html = root.join("foo.html");
    std::fs::write(&html, "<html></html>").unwrap();
    let targets = collect_format_targets(&html, None, true, true).unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].path, html);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_html_directory_to_separate_out_still_collects_html() {
    let _guard = format_test_lock();
    let root = temp_root("html-separate-out");
    let docs = root.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("foo.mec"), "x := 1").unwrap();
    std::fs::write(docs.join("foo.html"), "<html></html>").unwrap();
    let targets = collect_format_targets(&docs, None, true, false).unwrap();
    let names = targets.iter().map(|target| target.relative_path.clone()).collect::<Vec<_>>();
    assert!(names.contains(&PathBuf::from("foo.mec")));
    assert!(names.contains(&PathBuf::from("foo.html")));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_directory_dash_o_same_directory_collects_sources_in_place() {
    let _guard = format_test_lock();
    let root = temp_root("same-output-dir");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("docs/main.mec"), "x := 1").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let mech_paths = vec!["docs".to_string()];
    let output_matches_input_dir = format_output_matches_input_dir(&mech_paths, Path::new("docs"), false).unwrap();
    let writes_in_place = format_writes_in_place(Some("docs"), Path::new("docs"), false).unwrap() || output_matches_input_dir;
    let targets = collect_format_targets(Path::new("docs"), None, false, writes_in_place).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    assert!(writes_in_place);
    assert_eq!(targets.len(), 1);
    assert_eq!(format_output_file_for_target(&targets[0], Path::new("docs"), false, writes_in_place, false), PathBuf::from("docs/main.mec"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_directory_dash_o_same_directory_html_skips_generated_siblings() {
    let _guard = format_test_lock();
    let root = temp_root("same-output-html");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("docs/main.mec"), "x := 1").unwrap();
    std::fs::write(root.join("docs/main.html"), "<html></html>").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let mech_paths = vec!["docs".to_string()];
    let output_matches_input_dir = format_output_matches_input_dir(&mech_paths, Path::new("docs"), false).unwrap();
    let writes_in_place = format_writes_in_place(Some("docs"), Path::new("docs"), false).unwrap() || output_matches_input_dir;
    let targets = collect_format_targets(Path::new("docs"), None, true, writes_in_place).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    assert!(writes_in_place);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].path, PathBuf::from("docs/main.mec"));
    assert_eq!(format_output_file_for_target(&targets[0], Path::new("docs"), false, writes_in_place, true), PathBuf::from("docs/main.html"));
    ensure_unique_format_outputs(&targets, Path::new("docs"), false, writes_in_place, true).unwrap();
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_multiple_inputs_dash_o_matching_one_input_errors() {
    let _guard = format_test_lock();
    let root = temp_root("multi-same-output");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::create_dir_all(root.join("more")).unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let mech_paths = vec!["docs".to_string(), "more".to_string()];
    let output_matches_input_dir = format_output_matches_input_dir(&mech_paths, Path::new("docs"), false).unwrap();
    assert!(output_matches_input_dir);
    let error = format!("{:?}", reject_ambiguous_matching_output_dir(output_matches_input_dir, mech_paths.len(), Path::new("docs")).unwrap_err());
    assert!(error.contains("matches one of multiple format inputs"), "got {error}");
    std::env::set_current_dir(old_cwd).unwrap();
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_directory_dash_o_distinct_directory_still_writes_under_out() {
    let _guard = format_test_lock();
    let root = temp_root("distinct-output-dir");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("docs/main.mec"), "x := 1").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let targets = collect_format_targets(Path::new("docs"), None, false, false).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    assert_eq!(format_output_file_for_target(&targets[0], Path::new("out"), false, false, false), PathBuf::from("out/main.mec"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn reject_multi_target_format_to_single_file() {
    let _guard = format_test_lock();
    let root = temp_root("single-file-output");
    let docs = root.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("a.mec"), "a := 1").unwrap();
    std::fs::write(docs.join("b.mec"), "b := 2").unwrap();
    let targets = collect_format_targets(&docs, None, false, false).unwrap();
    let error = format!("{:?}", reject_multi_target_file_output(targets.len(), &root.join("out.mec"), true).unwrap_err());
    assert!(error.contains("Cannot write 2 formatted sources into single output file"), "got {error}");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn reject_multiple_explicit_inputs_to_single_file() {
    let _guard = format_test_lock();
    let root = temp_root("explicit-single-file-output");
    std::fs::write(root.join("a.mec"), "a := 1").unwrap();
    std::fs::write(root.join("b.mec"), "b := 2").unwrap();
    let mut targets = Vec::new();
    targets.extend(collect_format_targets(&root.join("a.mec"), None, false, false).unwrap());
    targets.extend(collect_format_targets(&root.join("b.mec"), None, false, false).unwrap());
    let error = format!("{:?}", reject_multi_target_file_output(targets.len(), &root.join("out.mec"), true).unwrap_err());
    assert!(error.contains("Cannot write 2 formatted sources into single output file"), "got {error}");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_format_targets_skips_selected_output_directory() {
    let _guard = format_test_lock();
    let root = temp_root("skip-selected-output");
    let docs = root.join("docs");
    let site = docs.join("site");
    std::fs::create_dir_all(&site).unwrap();
    std::fs::write(docs.join("main.mec"), "x := 1").unwrap();
    std::fs::write(site.join("main.html"), "<html></html>").unwrap();
    let exclusion = normalize_output_exclusion(&site, false).unwrap();
    let targets = collect_format_targets(&docs, exclusion.as_deref(), false, false).unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].relative_path, PathBuf::from("main.mec"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_format_targets_skips_markdown_in_directories_but_rejects_explicit_markdown() {
    let _guard = format_test_lock();
    let root = temp_root("markdown");
    std::fs::write(root.join("README.md"), "# Raw Markdown").unwrap();
    std::fs::write(root.join("demo.mec"), "x := 1").unwrap();

    let targets = collect_format_targets(&root, None, false, false).unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].relative_path, PathBuf::from("demo.mec"));

    let error = format!("{:?}", collect_format_targets(&root.join("README.md"), None, false, false).unwrap_err());
    assert!(error.contains("Unsupported source extension"), "got {error}");
    std::fs::remove_dir_all(root).unwrap();
  }
  #[test]
  fn format_explicit_absolute_file_default_updates_absolute_path() {
    let root = temp_root("absolute-default");
    let file = root.join("main.mec");
    std::fs::write(&file, "x := 1").unwrap();
    let targets = collect_format_targets(&file, None, false, true).unwrap();

    let output = format_output_file_for_target(&targets[0], Path::new("."), false, true, false);

    assert_eq!(output, file);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_explicit_absolute_file_out_dot_updates_absolute_path() {
    let root = temp_root("absolute-dot");
    let file = root.join("main.mec");
    std::fs::write(&file, "x := 1").unwrap();
    let targets = collect_format_targets(&file, None, false, true).unwrap();

    let output = format_output_file_for_target(&targets[0], Path::new("."), false, true, false);

    assert_eq!(output, file);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_explicit_absolute_file_out_directory_stays_inside_out() {
    let root = temp_root("absolute-out");
    let file = root.join("main.mec");
    std::fs::write(&file, "x := 1").unwrap();
    let targets = collect_format_targets(&file, None, false, false).unwrap();

    let output = format_output_file_for_target(&targets[0], Path::new("formatted"), false, false, false);

    assert_eq!(output, PathBuf::from("formatted").join("main.mec"));
    assert!(!output.components().any(|component| matches!(component, std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_))));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_explicit_absolute_file_html_default_writes_absolute_html_sibling() {
    let root = temp_root("absolute-html-default");
    let file = root.join("main.mec");
    std::fs::write(&file, "x := 1").unwrap();
    let targets = collect_format_targets(&file, None, true, true).unwrap();

    let output = format_output_file_for_target(&targets[0], Path::new("."), false, true, true);

    assert_eq!(output, root.join("main.html"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_explicit_absolute_file_html_out_directory_stays_inside_out() {
    let root = temp_root("absolute-html-out");
    let file = root.join("main.mec");
    std::fs::write(&file, "x := 1").unwrap();
    let targets = collect_format_targets(&file, None, true, false).unwrap();

    let output = format_output_file_for_target(&targets[0], Path::new("formatted"), false, false, true);

    assert_eq!(output, PathBuf::from("formatted").join("main.html"));
    assert!(!output.components().any(|component| matches!(component, std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_))));
    std::fs::remove_dir_all(root).unwrap();
  }

}

#[cfg(all(test, feature = "run"))]
mod run_collection_tests {
  use super::*;

  fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
      "mech-run-collection-{label}-{}",
      std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
  }

  #[test]
  fn collect_run_targets_accepts_explicit_mdoc() {
    let root = temp_root("explicit-mdoc");
    let doc = root.join("doc.mdoc");
    std::fs::write(&doc, "x := 1").unwrap();
    assert_eq!(collect_run_targets(&doc).unwrap(), vec![doc]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_discovers_mdoc_in_directory() {
    let root = temp_root("directory-mdoc");
    std::fs::write(root.join("doc.mdoc"), "x := 1").unwrap();
    std::fs::write(root.join("main.mec"), "y := 2").unwrap();
    let targets = collect_run_targets(&root).unwrap();
    assert!(targets.contains(&root.join("doc.mdoc")));
    assert!(targets.contains(&root.join("main.mec")));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_accepts_explicit_mpkg() {
    let root = temp_root("explicit-mpkg");
    let package = root.join("project.mpkg");
    std::fs::write(&package, "{}").unwrap();
    assert_eq!(collect_run_targets(&package).unwrap(), vec![package]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_discovers_mpkg_in_directory() {
    let root = temp_root("directory-mpkg");
    std::fs::write(root.join("project.mpkg"), "{}").unwrap();
    std::fs::write(root.join("main.mec"), "y := 2").unwrap();
    let targets = collect_run_targets(&root).unwrap();
    assert!(targets.contains(&root.join("project.mpkg")));
    assert!(targets.contains(&root.join("main.mec")));
    std::fs::remove_dir_all(root).unwrap();
  }


  #[test]
  fn collect_run_targets_accepts_explicit_m_source() {
    let root = temp_root("explicit-m");
    let source = root.join("script.m");
    std::fs::write(&source, "x := 1").unwrap();
    assert_eq!(collect_run_targets(&source).unwrap(), vec![source]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_accepts_explicit_csv_source() {
    let root = temp_root("explicit-csv");
    let source = root.join("data.csv");
    std::fs::write(&source, "x,y\n1,2\n").unwrap();
    assert_eq!(collect_run_targets(&source).unwrap(), vec![source]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_accepts_explicit_js_source() {
    let root = temp_root("explicit-js");
    let source = root.join("script.js");
    std::fs::write(&source, "console.log('mech');").unwrap();
    assert_eq!(collect_run_targets(&source).unwrap(), vec![source]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_ignores_loader_supported_text_sources_in_directory() {
    let root = temp_root("directory-loader-text");
    let m = root.join("script.m");
    let csv = root.join("data.csv");
    let js = root.join("script.js");
    std::fs::write(&m, "x := 1").unwrap();
    std::fs::write(&csv, "x,y\n1,2\n").unwrap();
    std::fs::write(&js, "console.log('mech');").unwrap();

    assert!(collect_run_targets(&root).unwrap().is_empty());
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_skips_mecb_in_directory() {
    let root = temp_root("directory-skip-mecb");
    let source = root.join("main.mec");
    let bytecode = root.join("output.mecb");
    std::fs::write(&source, "x := 1").unwrap();
    std::fs::write(&bytecode, b"bytecode").unwrap();

    let targets = collect_run_targets(&root).unwrap();

    assert_eq!(targets, vec![source]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_allows_explicit_mecb_file() {
    let root = temp_root("explicit-mecb");
    let bytecode = root.join("output.mecb");
    std::fs::write(&bytecode, b"bytecode").unwrap();

    assert_eq!(collect_run_targets(&bytecode).unwrap(), vec![bytecode]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_directory_only_includes_mech_source_document_package_extensions() {
    let root = temp_root("directory-run-supported");
    let files = vec![
      root.join("data.csv"),
      root.join("doc.mdoc"),
      root.join("main.mec"),
      root.join("project.mpkg"),
      root.join("script.js"),
      root.join("script.m"),
    ];
    for path in &files {
      std::fs::write(path, "x := 1").unwrap();
    }
    std::fs::write(root.join("output.mecb"), b"bytecode").unwrap();

    assert_eq!(collect_run_targets(&root).unwrap(), vec![
      root.join("doc.mdoc"),
      root.join("main.mec"),
      root.join("project.mpkg"),
    ]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_still_rejects_unsupported_extension() {
    let root = temp_root("unsupported");
    let source = root.join("notes.txt");
    std::fs::write(&source, "not a mech runtime source").unwrap();
    assert!(collect_run_targets(&source).is_err());
    std::fs::remove_dir_all(root).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn collect_run_targets_includes_symlinked_files_but_not_dirs_or_broken_links() {
    use std::os::unix::fs::symlink;
    let root = temp_root("symlink-file");
    std::fs::write(root.join("main.mec"), "x := 1").unwrap();
    symlink(root.join("main.mec"), root.join("linked.mec")).unwrap();
    symlink(&root, root.join("self")).unwrap();
    symlink(root.join("missing.mec"), root.join("broken.mec")).unwrap();

    let targets = collect_run_targets(&root).unwrap();
    assert_eq!(targets, vec![root.join("linked.mec"), root.join("main.mec")]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn collect_run_targets_skips_symlinked_mecb_in_directory() {
    use std::os::unix::fs::symlink;
    let root = temp_root("symlink-mecb");
    let source = root.join("main.mec");
    let bytecode = root.join("output.mecb");
    std::fs::write(&source, "x := 1").unwrap();
    std::fs::write(&bytecode, b"bytecode").unwrap();
    symlink(&bytecode, root.join("linked.mecb")).unwrap();

    let targets = collect_run_targets(&root).unwrap();

    assert_eq!(targets, vec![source]);
    std::fs::remove_dir_all(root).unwrap();
  }
}
