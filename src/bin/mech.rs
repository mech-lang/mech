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
        .help("Sets the stylesheet for the HTML output (include/style.css)"))
      .arg(Arg::new("shim")
        .short('m')
        .long("shim")
        .value_name("SHIM")
        .help("Sets the shim for the HTML output (include/shim.html)"))
      .arg(Arg::new("wasm")
        .short('w')
        .long("wasm")
        .value_name("WASM")
        .help("Sets the the path to the wasm package (src/wasm/pkg"))
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

  // --------------------------------------------------------------------------
  // Serve
  // --------------------------------------------------------------------------
  #[cfg(feature = "serve")]
  if let Some(matches) = matches.subcommand_matches("serve") {

    let port: String = matches.get_one::<String>("port").cloned().unwrap_or("8081".to_string());
    let address = matches.get_one::<String>("address").cloned().unwrap_or("127.0.0.1".to_string());
    let full_address: String = format!("{}:{}",address,port);
    let mech_paths: Vec<String> = matches.get_many::<String>("mech_serve_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let stylesheet = matches.get_one::<String>("stylesheet").cloned().unwrap_or("include/style.css".to_string());
    let wasm_pkg = matches.get_one::<String>("wasm").cloned().unwrap_or("src/wasm/pkg".to_string());
    let shim = matches.get_one::<String>("shim").cloned().unwrap_or("include/shim.html".to_string());

    if cfg!(feature = "serve") {
      let mut server = MechServer::new(full_address, stylesheet.to_string(), shim.to_string(), wasm_pkg.to_string());
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
    let html_flag = matches.get_flag("html");
    let stylesheet_url = matches.get_one::<String>("stylesheet").cloned().unwrap_or("https://gitlab.com/mech-lang/mech/-/raw/v0.2-beta/include/style.css?ref_type=heads".to_string());
    let shim_url = matches.get_one::<String>("shim").cloned().unwrap_or("https://gitlab.com/mech-lang/mech/-/raw/v0.2-beta/include/shim.html?ref_type=heads".to_string());
    let output_path = PathBuf::from(matches.get_one::<String>("output_path").cloned().unwrap_or(".".to_string()));

    let mech_paths: Vec<String> = matches.get_many::<String>("mech_format_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let mut mechfs = MechFileSystem::new();
    
    // open file or url. If it's a local file load it from disk, if it's a url fetch it from internet
    #[cfg(feature = "async")]
    let stylesheet = load_resource(&stylesheet_url).await;
    #[cfg(feature = "async")]
    let shim = load_resource(&shim_url).await;

    #[cfg(not(feature = "async"))]
    let stylesheet = load_resource(&stylesheet_url);
    #[cfg(not(feature = "async"))]
    let shim = load_resource(&shim_url);

    mechfs.set_stylesheet(&stylesheet);
    mechfs.set_shim(&shim);
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

    // Process files based on the flag
    if html_flag {
      for (fid, mech_src) in read_sources.html_iter() {
        if let MechSourceCode::Html(content) = mech_src {
          let mut filename = read_sources.get_path_from_id(*fid).unwrap().clone();
          filename = filename.with_extension("html");
          let output_file = output_path.join(filename);
          save_to_file(output_file, content)?;
        }
      }
    } else {
      for (fid, mech_src) in read_sources.sources_iter() {
        let content = mech_src.to_string();
        let filename = read_sources.get_path_from_id(*fid).unwrap().clone();
        let output_file = output_path.join(filename);
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
            println!("{} {:#?}",
              "[File Error]".truecolor(246,98,78),
              err
            );
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
          println!("{} {:#?}",
            "[Error]".truecolor(246,98,78),
            err
          );
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