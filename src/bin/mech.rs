#![feature(hash_extract_if)]
#![allow(warnings)]
use mech::*;
use mech_core::*;
#[cfg(feature = "run")]
use mech_runtime::RuntimeConfig;
#[cfg(feature = "serve")]
#[cfg(feature = "formatter")]
use mech_syntax::formatter::*;
use mech_syntax::parser;
use std::env;
use std::fs;
use std::io;
use std::time::Instant;

use ariadne::{Color, Label, Report, ReportKind, sources};
use clap::{Arg, ArgAction, Command};
use colored::*;
use crossterm::{ExecutableCommand, QueueableCommand, cursor, style::Print, terminal};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde_json;
use std::io::{BufReader, BufWriter, Write, stdout};
use std::panic;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tabled::{
    Tabled,
    builder::Builder,
    settings::{Alignment, Modify, Panel, Span, Style, object::Rows},
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

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
    pub source_error: String,
}
impl MechErrorKind for Utf8ConversionError {
    fn name(&self) -> &str {
        "Utf8ConversionError"
    }
    fn message(&self) -> String {
        format!(
            "Failed to convert bytes into UTF-8 string: {}",
            self.source_error
        )
    }
}

#[cfg(feature = "bundle_web")]
use mech::cli::bundle_web;
#[cfg(feature = "serve")]
use mech::cli::capabilities;

#[cfg(any(feature = "serve", feature = "run"))]
use mech::cli::config;
#[cfg(feature = "run")]
use mech::cli::run::{
    RunInputMode, classify_run_inputs, cli_host_capability_args,
    cli_host_capability_passthrough_values, cli_host_capability_selection,
    effective_run_runtime_config, new_cli_runtime, run_cli_source,
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
    command.subcommand(
        Command::new("serve")
            .about("Serve Mech program over an HTTP server.")
            .arg(
                Arg::new("mech_serve_file_paths")
                    .help("Source .mec and .mecb files")
                    .required(false)
                    .action(ArgAction::Append),
            )
            .arg(
                Arg::new("port")
                    .short('p')
                    .long("port")
                    .value_name("PORT")
                    .help("Sets the port for the server (8081)"),
            )
            .arg(
                Arg::new("stylesheet")
                    .short('s')
                    .long("stylesheet")
                    .value_name("STYLESHEET")
                    .num_args(1..)
                    .action(ArgAction::Append)
                    .help("Sets the stylesheet for the HTML output"),
            )
            .arg(
                Arg::new("shim")
                    .short('m')
                    .long("shim")
                    .value_name("SHIM")
                    .help("Sets the shim for the HTML output"),
            )
            .arg(
                Arg::new("wasm")
                    .short('w')
                    .long("wasm")
                    .value_name("WASM")
                    .help("Sets the the path to the wasm package"),
            )
            .arg(
                Arg::new("address")
                    .short('a')
                    .long("address")
                    .value_name("ADDRESS")
                    .help("Sets the address of the server (127.0.0.1)"),
            )
            .args(host_delegation_args()),
    )
}

#[cfg(not(feature = "serve"))]
fn add_serve_subcommand(command: Command) -> Command {
    command
}

async fn load_stylesheets(paths: &[String], fallback_url: &str) -> Result<String, MechError> {
    if paths.is_empty() {
        let stylesheet = read_or_download("", fallback_url, Some(STYLESHEET.as_bytes())).await?;
        return String::from_utf8(stylesheet).map_err(|e| {
            MechError::new(
                Utf8ConversionError {
                    source_error: e.to_string(),
                },
                None,
            )
            .with_compiler_loc()
        });
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
        let stylesheet_str = String::from_utf8(stylesheet).map_err(|e| {
            MechError::new(
                Utf8ConversionError {
                    source_error: e.to_string(),
                },
                None,
            )
            .with_compiler_loc()
        })?;
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
  └─┘     └─┘ └──────┘ └──────┘ └─┘  └─┘"#
        .truecolor(246, 192, 78);

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

    let micromika = "╭◉╮".truecolor(246, 192, 78);
    let micromika_point = "╭◉─".truecolor(246, 192, 78);
    let micromika_hello = "╭◉╯".truecolor(246, 192, 78);
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
        .arg(
            Arg::new("mech_paths")
                .help("Source .mec and files")
                .required(false)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Print debug info")
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("format")
                .about("Format Mech source code into standard format.")
                .arg(
                    Arg::new("mech_format_file_paths")
                        .help("Source .mec and .mecb files")
                        .required(false)
                        .action(ArgAction::Append),
                )
                .arg(
                    Arg::new("output_path")
                        .short('o')
                        .long("out")
                        .help("Destination folder.")
                        .required(false),
                )
                .arg(
                    Arg::new("stylesheet")
                        .short('s')
                        .long("stylesheet")
                        .value_name("STYLESHEET")
                        .num_args(1..)
                        .action(ArgAction::Append)
                        .help("Sets the stylesheet for the HTML output"),
                )
                .arg(
                    Arg::new("shim")
                        .short('m')
                        .long("shim")
                        .value_name("SHIM")
                        .help("Sets the shim for the HTML output"),
                )
                .arg(
                    Arg::new("html")
                        .short('t')
                        .long("html")
                        .required(false)
                        .help("Output as HTML")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("build")
                .about("Build Mech program into a binary.")
                .arg(
                    Arg::new("mech_build_file_paths")
                        .help("Source .mec and .mecb files")
                        .required(false)
                        .action(ArgAction::Append),
                )
                .arg(
                    Arg::new("debug")
                        .short('d')
                        .long("debug")
                        .help("Print debug info")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("output_path")
                        .short('o')
                        .long("out")
                        .help("Destination folder.")
                        .required(false),
                ),
        )
        .subcommand(
            Command::new("test")
                .about("Validate program invariants.")
                .arg(
                    Arg::new("mech_test_file_paths")
                        .help("Source .mec and .mecb files")
                        .required(false)
                        .action(ArgAction::Append),
                )
                .arg(
                    Arg::new("output_path")
                        .short('o')
                        .long("out")
                        .help("Write test output to .json or .mec.")
                        .required(false),
                )
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .help("Print verbose pass/fail details.")
                        .action(ArgAction::SetTrue)
                        .required(false),
                ),
        )
        .arg(
            Arg::new("tree")
                .short('e')
                .long("tree")
                .help("Print parse tree")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("time")
                .short('t')
                .long("time")
                .help("Measure how long the programs takes to execute.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("rounds-per-step")
                .long("rounds-per-step")
                .value_name("ROUNDS")
                .help("Sets the number of rounds per step (10_000)")
                .required(false),
        )
        .arg(
            Arg::new("trace")
                .long("trace")
                .help("Print trace output for state-machine arms and function calls")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("repl")
                .short('r')
                .long("repl")
                .help("Start REPL")
                .action(ArgAction::SetTrue),
        );

    let cli_command = add_run_subcommand(cli_command);
    let cli_command = add_serve_subcommand(cli_command);
    let cli_command = add_cli_host_capability_args(cli_command);

    #[cfg(feature = "bundle_web")]
    let cli_command = cli_command.subcommand(bundle_web::bundle_web_command());

    #[cfg(feature = "serve")]
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
    let root_rounds_per_step = cli_matches
        .get_one::<String>("rounds-per-step")
        .and_then(|s| s.parse::<usize>().ok());

    let shim_backup_url =
        "https://raw.githubusercontent.com/mech-lang/mech/refs/heads/main/include/shim.html"
            .to_string();
    let stylesheet_backup_url =
        "https://raw.githubusercontent.com/mech-lang/mech/refs/heads/main/include/style.css"
            .to_string();
    let wasm_backup_url = format!(
        "https://github.com/mech-lang/mech/releases/download/v{}-beta/mech_wasm_bg.wasm.br",
        VERSION
    );
    let js_backup_url = format!(
        "https://github.com/mech-lang/mech/releases/download/v{}-beta/mech_wasm.js",
        VERSION
    );

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
        let host_config = loaded_config.as_ref().map(|loaded| {
            mech_host_browser::BrowserHostConfig::from_document_and_runtime(
                &loaded.document,
                &runtime_config,
            )
        });
        let config_shim_at_root = loaded_config
            .as_ref()
            .and_then(|loaded| loaded.document.serve.as_ref())
            .and_then(|serve| serve.shim.as_ref())
            .is_some()
            && serve_matches.get_one::<String>("shim").is_none();
        if let Some(loaded) = loaded_config.as_ref() {
            println!(
                "{badge} Loaded browser config grants: {}",
                loaded.document.browser.grants().len()
            );
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
        let shim = read_or_download(shim_path, &shim_backup_url, Some(SHIMHTML.as_bytes())).await?;

        let shim_str = String::from_utf8(shim).map_err(|e| {
            MechError::new(
                Utf8ConversionError {
                    source_error: e.to_string(),
                },
                None,
            )
            .with_compiler_loc()
        })?;

        print!("{badge} Loading WASM…");
        let wasm = read_or_download(&wasm_path, &wasm_backup_url, Some(MECHWASM)).await?;

        print!("{badge} Loading JS…");
        let js = read_or_download(&js_path, &js_backup_url, Some(MECHJS)).await?;

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
    #[cfg(all(
        feature = "run",
        feature = "variable_define",
        feature = "invariant_define",
        feature = "symbol_table",
        feature = "bool"
    ))]
    if let Some(matches) = cli_matches.subcommand_matches("test") {
        let mech_paths: Vec<String> = matches
            .get_many::<String>("mech_test_file_paths")
            .map_or(vec![".".to_string()], |files| {
                files.map(|file| file.to_string()).collect()
            });
        let output_path = matches.get_one::<String>("output_path").cloned();
        let verbose = matches.get_flag("verbose");
        let exit_code = run_mech_tests(
            mech_paths,
            tree_flag,
            debug_flag,
            time_flag,
            trace_flag,
            output_path,
            verbose,
        )?;
        std::process::exit(exit_code);
    }

    // --------------------------------------------------------------------------
    // Build
    // --------------------------------------------------------------------------
    #[cfg(feature = "build")]
    if let Some(matches) = cli_matches.subcommand_matches("build") {
        let mech_paths: Vec<String> = matches
            .get_many::<String>("mech_build_file_paths")
            .map_or(vec![], |files| files.map(|file| file.to_string()).collect());
        let output_path = PathBuf::from(
            matches
                .get_one::<String>("output_path")
                .cloned()
                .unwrap_or(".".to_string()),
        );
        let debug_flag = matches.get_flag("debug");
        let rounds_per_step = matches
            .get_one::<String>("rounds-per-step")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(10_000);
        // Create the directory html_output_path
        if output_path != PathBuf::from(".") {
            match fs::create_dir_all(&output_path) {
                Ok(_) => {
                    println!(
                        "{} Directory created: {}",
                        "[Created]".truecolor(153, 221, 85),
                        output_path.display()
                    );
                }
                Err(err) => {
                    println!("Error creating directory: {:?}", err);
                }
            }
        }

        let uuid = generate_uuid();
        let mut program = MechProgram::new(MechProgramConfig {
            name: format!("program-{}", uuid),
            environment: MechProgramEnvironment::default(),
        });
        let _ = tree_flag;
        let _ = trace_flag;
        program.configure(debug_flag, trace_flag, time_flag, rounds_per_step);
        for path in mech_paths {
            let source = std::fs::read_to_string(&path)?;
            let _ = program.run_string(&source)?;
        }

        let bytecode = program.interpreter_mut().compile()?;

        let mut output_file = output_path.join("output.mecb");

        let mut f = std::fs::File::create(&output_file)?;
        f.write_all(&bytecode)?;
        f.flush()?;

        // print debug info for the context
        if debug_flag {
            println!(
                "{} Bytecode Size: {:#?} bytes",
                "[Debug]".truecolor(246, 192, 78),
                &program.interpreter().context
            );
        }

        println!(
            "{} Mech bytecode written to: {}",
            "[Output]".truecolor(153, 221, 85),
            output_file.display()
        );

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

        let output_path = PathBuf::from(
            matches
                .get_one::<String>("output_path")
                .cloned()
                .unwrap_or(".".to_string()),
        );
        let is_output_file = output_path.extension().is_some();

        let mech_paths: Vec<String> = matches
            .get_many::<String>("mech_format_file_paths")
            .map_or(vec![], |files| files.map(|file| file.to_string()).collect());

        // If the user provided exactly one path
        if mech_paths.len() == 1 {
            let input_path = PathBuf::from(&mech_paths[0]);
            if input_path.is_dir() && is_output_file {
                eprintln!(
                    "{} Cannot write directory `{}` into single output file `{}`. Provide a directory for --out instead.",
                    "[Error]".truecolor(246, 98, 78),
                    input_path.display(),
                    output_path.display()
                );
                return Ok(());
            }
        }
        println!("{} Loading resources…", badge);

        // Load stylesheet
        print!("{} Loading stylesheet…", badge);
        let stylesheet_str = load_stylesheets(&stylesheet_paths, &stylesheet_backup_url).await?;

        // Load shim HTML
        print!("{} Loading HTML shim…", badge);
        let shim =
            read_or_download(&shim_path, &shim_backup_url, Some(SHIMHTML.as_bytes())).await?;
        let shim_str = String::from_utf8(shim).map_err(|e| {
            MechError::new(
                Utf8ConversionError {
                    source_error: e.to_string(),
                },
                None,
            )
            .with_compiler_loc()
        })?;

        let mut loaded_sources: Vec<(PathBuf, MechSourceCode)> = Vec::new();
        for path in mech_paths {
            let pb = PathBuf::from(&path);
            let source = std::fs::read_to_string(&pb)?;
            let code = match pb.extension().and_then(|e| e.to_str()) {
                Some("html") => MechSourceCode::Html(source),
                _ => MechSourceCode::String(source),
            };
            loaded_sources.push((pb, code));
        }

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
            let html_items: Vec<_> = loaded_sources
                .iter()
                .filter_map(|(p, src)| {
                    if let MechSourceCode::Html(content) = src {
                        Some((p, content))
                    } else {
                        None
                    }
                })
                .collect();
            let is_single_html = html_items.len() == 1;

            if is_output_file && is_single_html {
                // write ONLY HTML result to output file
                let (_, content) = html_items[0];
                save_to_file(output_path, content)?;
            } else {
                // otherwise produce multiple output files
                for (path, content) in html_items {
                    let filename = path.with_extension("html");
                    let output_file = if is_output_file {
                        output_path.clone()
                    } else {
                        output_path.join(filename)
                    };
                    save_to_file(output_file, content)?;
                }
            }
        } else {
            // Raw source mode
            for (filename, mech_src) in loaded_sources {
                let content = mech_src.to_string();
                let output_file = if is_output_file {
                    output_path.clone()
                } else {
                    output_path.join(filename)
                };
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
        run_inputs.extend(cli_host_capability_passthrough_values(
            &cli_matches,
            run_matches,
        ));

        let run_debug_flag =
            debug_flag || run_matches.map(|m| m.get_flag("debug")).unwrap_or(false);
        let run_trace_flag =
            trace_flag || run_matches.map(|m| m.get_flag("trace")).unwrap_or(false);
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
        let cli_grants =
            config::effective_cli_host_grants(loaded_config.as_ref(), cli_capability_selection)?;

        let mut runtime = new_cli_runtime(runtime_config, &cli_grants)?;

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
                    println!("{} {:#?}", "[Error]".truecolor(246, 98, 78), err);
                    std::process::exit(1);
                }
            }
        }

        let run_paths = match run_input_mode {
            RunInputMode::Paths(paths) => paths,
            RunInputMode::Empty => Vec::new(),
            RunInputMode::InlineSource(_) => {
                unreachable!("inline source exits before path execution")
            }
        };

        let options =
            config::effective_run_options(run_paths, loaded_config.as_ref(), explicit_run_command)?;

        let result: MResult<Value> = if let Some(options) = options {
            let mut last = Value::Empty;
            for p in &options.paths {
                let src = std::fs::read_to_string(p)?;
                last = run_cli_source(&mut runtime, &src)?;
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
        println!(
            "\n                {}                ",
            format!("v{}", VERSION).truecolor(246, 192, 78)
        );
        println!("           {}           \n", "www.mech-lang.org");
        let intro_message = format!(
            "{}Enter {} for a list of all commands.{}\n",
            mika_open, help_cmd, mika_close
        );
        println!("{} {}", micromika, intro_message);

        // Catch Ctrl-C a couple times before quitting
        let mut ci = caught_inturrupts.clone();
        ctrlc::set_handler(move || {
            println!("{}", ctrlc_cmd);
            let mut caught_inturrupts = ci.lock().unwrap();
            *caught_inturrupts += 1;
            if *caught_inturrupts >= 3 {
                let final_state = ProgressBar::new_spinner();
                let completed_style = ProgressStyle::with_template("\n{spinner:.yellow} {msg}")
                    .unwrap()
                    .tick_strings(MICROMIKA_WAVE);
                final_state.set_style(completed_style);
                final_state.set_message(format!("{}Okay cya!{}\n", mika_open, mika_close));
                for _ in 0..MICROMIKA_WAVE.len() - 1 {
                    thread::sleep(Duration::from_millis(100));
                    final_state.tick();
                }
                std::process::exit(0);
            }
            println!(
                "\n{} {}Enter {} to terminate this REPL session.{}\n",
                micromika_point, mika_open, quit_cmd, mika_close
            );
            print_prompt();
        })
        .expect("Error setting Ctrl+C handler");
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
                Ok((_, repl_command)) => match repl.execute_repl_command(repl_command) {
                    Ok(output) => {
                        println!("{}", output);
                    }
                    Err(err) => {
                        println!("!{:?}", err);
                    }
                },
                Err(x) => {
                    println!(
                        "{} Unrecognized command: {}",
                        "[Error]".truecolor(246, 98, 78),
                        x
                    );
                }
            }
        } else if input.trim() == "" {
            continue;
        } else {
            let cmd = ReplCommand::Code(vec![("repl".to_string(), MechSourceCode::String(input))]);
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
        Arg::new("host_delegation_key")
            .long("host-delegation-key")
            .value_name("PATH")
            .num_args(1),
        Arg::new("host_delegation_public_key")
            .long("host-delegation-public-key")
            .value_name("PATH")
            .num_args(1),
        Arg::new("host_delegation_key_id")
            .long("host-delegation-key-id")
            .value_name("ID")
            .num_args(1),
        Arg::new("host_delegation_issuer")
            .long("host-delegation-issuer")
            .value_name("ISSUER")
            .num_args(1),
        Arg::new("host_delegation_subject")
            .long("host-delegation-subject")
            .value_name("SUBJECT")
            .num_args(1),
        Arg::new("host_delegation_audience")
            .long("host-delegation-audience")
            .value_name("AUDIENCE")
            .num_args(1),
        Arg::new("host_delegation_expires_ms")
            .long("host-delegation-expires-ms")
            .value_name("MS")
            .num_args(1),
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
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "--host-delegation-public-key is required with --host-delegation-key",
            )
        })?;
    let Some(loaded_config) = loaded_config else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "host delegation signing requires a loaded config",
        )
        .into());
    };
    let current_dir = std::env::current_dir()?;
    let options = mech::HostDelegationSigningOptions {
        private_key_path: current_dir.join(private_key),
        public_key_path: current_dir.join(public_key),
        key_id: matches
            .get_one::<String>("host_delegation_key_id")
            .cloned()
            .unwrap_or_else(|| "dev".to_string()),
        issuer: matches
            .get_one::<String>("host_delegation_issuer")
            .cloned()
            .unwrap_or_else(|| "host://mech-cli".to_string()),
        subject: matches
            .get_one::<String>("host_delegation_subject")
            .cloned()
            .unwrap_or_else(|| "wasm://browser".to_string()),
        audience: matches
            .get_one::<String>("host_delegation_audience")
            .cloned()
            .unwrap_or_else(|| format!("browser://serve/{full_address}")),
        expires_ms: matches
            .get_one::<String>("host_delegation_expires_ms")
            .map(|value| value.parse())
            .transpose()
            .map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "--host-delegation-expires-ms must be an integer",
                )
            })?,
    };
    let host_config = mech_host_browser::BrowserHostConfig::from_document_and_runtime(
        &loaded_config.document,
        runtime_config,
    );
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))?
        .as_millis() as u64;
    mech::signed_browser_host_config_injection(host_config, &options, now_ms).map(Some)
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
    while start_offset < end_offset && file_content.as_bytes()[start_offset].is_ascii_whitespace() {
        start_offset += 1;
    }
    while end_offset > start_offset && file_content.as_bytes()[end_offset - 1].is_ascii_whitespace()
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
                    let first_error_range = report
                        .1
                        .first()
                        .map(|e| e.cause_rng.clone())
                        .unwrap_or(SourceRange::default());
                    let (first_start, first_end) =
                        source_range_to_offset_range(&report.0, &first_error_range);
                    let mut error_report = Report::build(
                        ReportKind::Error,
                        (src_file_path.clone(), first_start..first_end),
                    )
                    .with_message(format!("Syntax Errors Found: {}", report.1.len()));

                    for (err_num, err_ctx) in report.1.iter().enumerate() {
                        let (start, end) =
                            source_range_to_offset_range(&report.0, &err_ctx.cause_rng);

                        if let Some(annotation_rng) = err_ctx.annotation_rngs.first() {
                            let (ann_start, ann_end) =
                                source_range_to_offset_range(&report.0, annotation_rng);

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
    use mech_runtime::{
        DefaultIdGenerator, FS_IMPORT, FS_LIST, FS_READ, FS_RESOLVE, FS_SERVE, FS_WATCH,
        SERVE_HOST_SUBJECT,
    };

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
        let authority =
            capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
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
        let authority =
            capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
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
        let authority =
            capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
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
        let authority =
            capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
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
        let authority =
            capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
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
        let authority =
            capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
        let mut ids = DefaultIdGenerator::new();
        authority
            .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &root, true, [FS_SERVE])
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
