#![feature(hash_extract_if)]
#![allow(warnings)]
use mech::*;
use mech_core::*;
use mech_syntax::parser;
#[cfg(feature = "formatter")]
use mech_syntax::formatter::*;
use mech_interpreter::interpreter::*;
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
use clap::{arg, command, value_parser, Arg, ArgAction, Command};
use std::path::PathBuf;
use tabled::{
  builder::Builder,
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};
use serde_json;
use std::panic;
use std::sync::{Arc, Mutex};
use std::path::Path;
use std::ffi::OsStr;

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

#[tokio::main]
async fn main() -> Result<(), MechError2> {
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


  let about = format!("{}", text_logo);

  let matches = Command::new("Mech")
    .version(VERSION)
    .author("Corey Montella corey@mech-lang.org")
    .about(about)
    .arg(Arg::new("mech_paths")
        .help("Source .mec and files")
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
        .help("Source .mec and .mecb files")
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
    .subcommand(Command::new("serve")
      .about("Serve Mech program over an HTTP server.")
      .arg(Arg::new("mech_serve_file_paths")
        .help("Source .mec and .mecb files")
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
        .help("Sets the address of the server (127.0.0.1)")))
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
    .arg(Arg::new("repl")
        .short('r')
        .long("repl")
        .help("Start REPL")
        .action(ArgAction::SetTrue))
    .get_matches();

  let debug_flag = matches.get_flag("debug");
  let tree_flag = matches.get_flag("tree");
  let mut repl_flag = matches.get_flag("repl");
  let time_flag = matches.get_flag("time");

  let shim_backup_url = "https://raw.githubusercontent.com/mech-lang/mech/refs/heads/main/include/shim.html".to_string();
  let stylesheet_backup_url = "https://raw.githubusercontent.com/mech-lang/mech/refs/heads/main/include/style.css".to_string();
  let wasm_backup_url = format!("https://github.com/mech-lang/mech/releases/download/v{}-beta/mech_wasm_bg.wasm.br", VERSION);
  let js_backup_url = format!("https://github.com/mech-lang/mech/releases/download/v{}-beta/mech_wasm.js", VERSION);

  // --------------------------------------------------------------------------
  // Serve
  // --------------------------------------------------------------------------
  #[cfg(feature = "serve")]
  if let Some(matches) = matches.subcommand_matches("serve") {
    let badge = "[Mech Server]".truecolor(34, 204, 187);
    
    let port: String = matches.get_one::<String>("port").cloned().unwrap_or("8081".to_string());
    let address = matches.get_one::<String>("address").cloned().unwrap_or("127.0.0.1".to_string());
    let full_address: String = format!("{}:{}",address,port);
    let mech_paths: Vec<String> = matches.get_many::<String>("mech_serve_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let stylesheet_path = matches.get_one::<String>("stylesheet").cloned().unwrap_or("".to_string());
    let wasm_pkg = matches.get_one::<String>("wasm").cloned().unwrap_or("".to_string());
    let shim_path = matches.get_one::<String>("shim").cloned().unwrap_or("".to_string());

    let wasm_path = format!("{}/mech_wasm_bg.wasm.br", wasm_pkg);
    let js_path = format!("{}/mech_wasm.js", wasm_pkg);

    // Load stylesheet
    println!("{} Loading resources...", badge);
    print!("{} Loading stylesheet...", badge);
    let stylesheet = read_or_download(&stylesheet_path, &stylesheet_backup_url, Some(STYLESHEET.as_bytes()))
        .await?;
    let stylesheet_str = String::from_utf8(stylesheet)
        .map_err(|e| MechError2::new(Utf8ConversionError { source_error: e.to_string() }, None).with_compiler_loc())?;

    // Load shim HTML
    print!("{} Loading HTML shim...", badge);
    let shim = read_or_download(&shim_path, &shim_backup_url, Some(SHIMHTML.as_bytes()))
        .await?;
    let shim_str = String::from_utf8(shim)
        .map_err(|e| MechError2::new(Utf8ConversionError { source_error: e.to_string() }, None).with_compiler_loc())?;

    // Load WASM
    print!("{} Loading WASM...", badge);
    let wasm = read_or_download(&wasm_path, &wasm_backup_url, Some(MECHWASM))
        .await?;

    // Load JS shim
    print!("{} Loading JS...", badge);
    let js = read_or_download(&js_path, &js_backup_url, Some(MECHJS))
        .await?;

    if cfg!(feature = "serve") {
      let mut server = MechServer::new("Mech Server".to_string(), full_address, stylesheet_str, shim_str, wasm, js);
      #[cfg(feature = "serve")]
      server.init().await?;
      #[cfg(feature = "serve")]
      server.load_sources(&mech_paths)?;
      #[cfg(feature = "serve")]
      server.serve().await?;
    }    
  }
  
  // --------------------------------------------------------------------------
  // Build
  // --------------------------------------------------------------------------
  #[cfg(feature = "build")]
  if let Some(matches) = matches.subcommand_matches("build") {
    let mech_paths: Vec<String> = matches.get_many::<String>("mech_build_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let output_path = PathBuf::from(matches.get_one::<String>("output_path").cloned().unwrap_or(".".to_string()));
    let debug_flag = matches.get_flag("debug");
    let mut mechfs = MechFileSystem::new();

    for path in mech_paths {
      mechfs.watch_source(&path)?;
    }
    let sources = mechfs.sources();
    let read_sources = sources.read().unwrap();

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

    let uuid = generate_uuid();
    let mut intrp = Interpreter::new(uuid);

    let result = run_mech_code(&mut intrp, &mechfs, tree_flag, debug_flag, time_flag); 

    let bytecode = intrp.compile()?;

    let mut output_file = output_path.join("output.mecb");

    let mut f = std::fs::File::create(&output_file)?;
    f.write_all(&bytecode)?;
    f.flush()?;

    // print debug info for the context
    if debug_flag {
      println!("{} Bytecode Size: {:#?} bytes", "[Debug]".truecolor(246,192,78), intrp.context);
    }

    println!("{} Mech bytecode written to: {}", "[Output]".truecolor(153,221,85), output_file.display());

    return Ok(());
  }

  // --------------------------------------------------------------------------
  // Format
  // --------------------------------------------------------------------------
  #[cfg(feature = "formatter")]
  if let Some(matches) = matches.subcommand_matches("format") {
    let badge = "[Mech Formatter]".truecolor(34, 204, 187);
    let html_flag = matches.get_flag("html");

    let stylesheet_path = matches
        .get_one::<String>("stylesheet")
        .cloned()
        .unwrap_or("".to_string());

    let shim_path = matches
        .get_one::<String>("shim")
        .cloned()
        .unwrap_or("".to_string());

    let output_path =
        PathBuf::from(matches.get_one::<String>("output_path").cloned().unwrap_or(".".to_string()));
    let is_output_file = output_path.extension().is_some();

    let mech_paths: Vec<String> = matches
        .get_many::<String>("mech_format_file_paths")
        .map_or(vec![], |files| files.map(|file| file.to_string()).collect());

    let mut mechfs = MechFileSystem::new();

    println!("{} Loading resources...", badge);

    // Load stylesheet
    print!("{} Loading stylesheet...", badge);
    let stylesheet = read_or_download(&stylesheet_path, &stylesheet_backup_url, Some(STYLESHEET.as_bytes()))
            .await?;
    let stylesheet_str = String::from_utf8(stylesheet).map_err(|e| {
      MechError2::new(
        Utf8ConversionError {
          source_error: e.to_string(),
        },
        None,
      )
      .with_compiler_loc()
    })?;

    // Load shim HTML
    print!("{} Loading HTML shim...", badge);
    let shim = read_or_download(&shim_path, &shim_backup_url, Some(SHIMHTML.as_bytes())).await?;
    let shim_str = String::from_utf8(shim).map_err(|e| {
      MechError2::new(
        Utf8ConversionError {
          source_error: e.to_string(),
        },
        None,
      )
      .with_compiler_loc()
    })?;

    mechfs.set_stylesheet(&stylesheet_str);
    mechfs.set_shim(&shim_str);

    for path in mech_paths {
      mechfs.watch_source(&path)?;
    }

    let sources = mechfs.sources();
    let read_sources = sources.read().unwrap();

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
      let html_items: Vec<_> = read_sources.html_iter().collect();
      let is_single_html = html_items.len() == 1;

      if is_output_file && is_single_html {
        // write ONLY HTML result to output file
        let (_, mech_src) = html_items[0];
        if let MechSourceCode::Html(content) = mech_src {
          save_to_file(output_path, content)?;
        }
      } else {
        // otherwise produce multiple output files
        for (fid, mech_src) in html_items {
          if let MechSourceCode::Html(content) = mech_src {
            let mut filename = read_sources.get_path_from_id(*fid).unwrap().clone();
            filename = filename.with_extension("html");
            let output_file = if is_output_file { output_path.clone() } 
                              else { output_path.join(filename) };
            save_to_file(output_file, content)?;
          }
        }
      }
    } else {
      // Raw source mode
      for (fid, mech_src) in read_sources.sources_iter() {
        let content = mech_src.to_string();
        let filename = read_sources.get_path_from_id(*fid).unwrap().clone();
        let output_file = if is_output_file { output_path.clone() } 
                          else { output_path.join(filename) };
        save_to_file(output_file, &content)?;
      }
    }

    return Ok(());
  }


  // --------------------------------------------------------------------------
  // Run
  // --------------------------------------------------------------------------
  let mut caught_inturrupts = Arc::new(Mutex::new(0));
  let uuid = generate_uuid();
  let mut intrp = Interpreter::new(uuid);
  #[cfg(feature = "run")]
  {
    let mut paths = if let Some(m) = matches.get_many::<String>("mech_paths") {
      m.map(|s| s.to_string()).collect()
    } else { repl_flag = true; vec![] };

    let mut mechfs = MechFileSystem::new();

    let any_look_like_paths = paths.iter().any(|p| {
      is_intended_path(p)
    });

    if !paths.is_empty() {
      if any_look_like_paths {
        let mut watch_errors = Vec::new();
        for p in &paths {
          match mechfs.watch_source(p) {
            Ok(r) => {}
            Err(err) => watch_errors.push(err),
          }
        }
        if !watch_errors.is_empty() {
          // These looked like paths but failed to watch
          // Print errors
          for err in &watch_errors {
            print_mech_error(err);
          }
          std::process::exit(1);
        }
      } else {
        // ---------- 4. Treat the inputs as Mech code ----------
        intrp.clear();
        let joined = paths.join(" ");
        let parse_result = parser::parse(joined.trim());

        match parse_result {
          Ok(tree) => match intrp.interpret(&tree) {
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
          },

          Err(err) => {
            println!("{} {:#?}",
              "[Parse Error]".truecolor(246,98,78),
              err
            );
            std::process::exit(1);
          }
        }
      }
    }

    let result = run_mech_code(&mut intrp, &mechfs, tree_flag, debug_flag, time_flag); 
    if !repl_flag {
      match &result {
        Ok(ref r) => {
          println!("{}", r.kind());
          #[cfg(feature = "pretty_print")]
          println!("{}", r.pretty_print());
          #[cfg(not(feature = "pretty_print"))]
          println!("{:#?}", r);
          std::process::exit(0);
        }
        Err(ref err) => {
          print_mech_error(err);
          std::process::exit(1);
        }
      };
    }
    
    #[cfg(windows)]
    control::set_virtual_terminal(true).unwrap();
    clc();
    let mut stdo = stdout();
    stdo.execute(Print(text_logo));
    stdo.execute(cursor::MoveToNextLine(1));
    println!("\n                {}                ",format!("v{}",VERSION).truecolor(246,192,78));
    println!("           {}           \n", "www.mech-lang.org");
    println!("Enter \":help\" for a list of all commands.\n");

    // Catch Ctrl-C a couple times before quitting
    let mut ci = caught_inturrupts.clone();
    ctrlc::set_handler(move || {
      println!("[Ctrl+C]");
      let mut caught_inturrupts = ci.lock().unwrap();
      *caught_inturrupts += 1;
      if *caught_inturrupts >= 3 {
        println!("Okay, cya!");
        std::process::exit(0);
      }
      println!("Enter \":quit\" to terminate this REPL session.");
      print_prompt();
    }).expect("Error setting Ctrl-C handler");
  }

  // --------------------------------------------------------------------------
  // REPL
  // --------------------------------------------------------------------------
  #[cfg(feature = "repl")]
  let mut repl = MechRepl::from(intrp);
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
          println!("!!{:?}", err);
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

fn is_intended_path(s: &str) -> bool {
  if s.trim().is_empty() { return false; }

  let path = Path::new(s);
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

pub fn print_mech_error(err: &MechError2) {
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
