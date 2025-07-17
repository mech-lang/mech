#![feature(hash_extract_if)]
#![allow(warnings)]
extern crate tokio;
use mech::*;
use mech_core::*;
use mech_syntax::parser;
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


const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<(), MechError> {
  /*panic::set_hook(Box::new(|panic_info| {
    // do nothing.
  }));*/

  let text_logo = r#"
  ┌─────────┐ ┌──────┐ ┌─┐ ┌──┐ ┌─┐   ┌─┐
  └───┐ ┌───┘ └──────┘ │ │ └┐ │ │ │   │ │
  ┌─┐ │ │ ┌─┐ ┌──────┐ │ │  └─┘ │ └─┐ │ │
  │ │ │ │ │ │ │ ┌────┘ │ │  ┌─┐ │ ┌─┘ │ │
  │ │ └─┘ │ │ │ └────┐ │ └──┘ │ │ │   │ │
  └─┘     └─┘ └──────┘ └──────┘ └─┘   └─┘"#.truecolor(246,192,78);


  let super_3D_logo = r#"
          _____                      _____                      _____                      _____         
         ╱╲    ╲                    ╱╲    ╲                    ╱╲    ╲                    ╱╲    ╲         
        ╱┊┊╲    ╲                  ╱┊┊╲    ╲                  ╱┊┊╲____╲                  ╱┊┊╲____╲        
        ╲┊┊┊╲    ╲                 ╲┊┊┊╲    ╲                ╱┊┊┊╱    ╱                 ╱┊┊┊╱    ╱        
      ___╲┊┊┊╲    ╲              ___╲┊┊┊╲    ╲              ╱┊┊┊╱   _╱___              ╱┊┊┊╱    ╱         
     ╱╲   ╲┊┊┊╲    ╲            ╱╲   ╲┊┊┊╲    ╲            ╱┊┊┊╱   ╱╲    ╲            ╱┊┊┊╱    ╱          
    ╱┊┊╲___╲┊┊┊╲    ╲          ╱┊┊╲   ╲┊┊┊╲    ╲          ╱┊┊┊╱   ╱┊┊╲    ╲          ╱┊┊┊╱___ ╱           
   ╱┊┊┊╱   ╱┊┊┊┊╲    ╲        ╱┊┊┊┊╲   ╲┊┊┊╲    ╲        ╱┊┊┊╱    ╲┊┊┊╲    ╲        ╱┊┊┊┊╲    ╲           
  ╱┊┊┊╱   ╱┊┊┊┊┊┊╲    ╲      ╱┊┊┊┊┊┊╲   ╲┊┊┊╲    ╲      ╱┊┊┊╱    ╱ ╲┊┊┊╲    ╲      ╱┊┊┊┊┊┊╲    ╲   _____  
 ╱┊┊┊╱   ╱┊┊┊╱╲┊┊┊╲    ╲    ╱┊┊┊╱╲┊┊┊╲   ╲┊┊┊╲    ╲    ╱┊┊┊╱    ╱   ╲┊┊┊╲    ╲    ╱┊┊┊╱╲┊┊┊╲____╲ ╱╲    ╲ 
╱┊┊┊╱   ╱┊┊┊╱  ╲┊┊┊╲____╲  ╱┊┊┊╱__╲┊┊┊╲   ╲┊┊┊╲____╲  ╱┊┊┊╱____╱     ╲┊┊┊╲____╲  ╱┊┊┊╱  ╲┊┊╱    ╱╱┊┊╲____╲
╲┊┊╱   ╱┊┊┊╱    ╲┊┊╱    ╱  ╲┊┊┊╲   ╲┊┊┊╲   ╲┊┊╱    ╱  ╲┊┊┊╲    ╲     ╱┊┊┊╱    ╱  ╲┊┊╱    ╲╱____╱╱┊┊┊╱    ╱
 ╲╱__ ╱┊┊┊╱   ___╲╱____╱    ╲┊┊┊╲   ╲┊┊┊╲   ╲╱____╱    ╲┊┊┊╲    ╲    ╲┊┊╱    ╱    ╲╱____╱      ╱┊┊┊╱    ╱ 
     ╱┊┊┊╱   ╱╲    ╲         ╲┊┊┊╲   ╲┊┊┊╲    ╲         ╲┊┊┊╲    ╲  __╲╱___ ╱                 ╱┊┊┊╱    ╱  
    ╱┊┊┊╱   ╱┊┊╲____╲         ╲┊┊┊╲   ╲┊┊┊╲____╲         ╲┊┊┊╲    ╱╲    ╲                    ╱┊┊┊╱    ╱   
    ╲┊┊╱   ╱┊┊┊╱    ╱          ╲┊┊┊╲   ╲┊┊╱    ╱          ╲┊┊┊╲  ╱┊┊╲____╲                  ╱┊┊┊╱    ╱    
     ╲╱__ ╱┊┊┊╱    ╱            ╲┊┊┊╲   ╲╱____╱            ╲┊┊┊╲╱┊┊┊╱    ╱                 ╱┊┊┊╱    ╱     
         ╱┊┊┊╱    ╱              ╲┊┊┊╲    ╲                 ╲┊┊┊┊┊┊╱    ╱                 ╱┊┊┊╱    ╱      
        ╱┊┊┊╱    ╱                ╲┊┊┊╲____╲                 ╲┊┊┊┊╱    ╱                 ╱┊┊┊╱    ╱       
        ╲┊┊╱    ╱                  ╲┊┊╱    ╱                  ╲┊┊╱    ╱                  ╲┊┊╱    ╱        
         ╲╱____╱                    ╲╱____╱                    ╲╱____╱                    ╲╱____╱"#.truecolor(246,192,78);



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
        .help("Source .mec and .blx files")
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
      .arg(Arg::new("html")
        .short('t')
        .long("html")
        .required(false)
        .help("Output as HTML")
        .action(ArgAction::SetTrue)))
    .subcommand(Command::new("serve")
      .about("Serve Mech program over an HTTP server.")
      .arg(Arg::new("mech_serve_file_paths")
        .help("Source .mec and .blx files")
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

  let uuid = generate_uuid();
  let mut intrp = Interpreter::new(uuid);

  // --------------------------------------------------------------------------
  // Serve
  // --------------------------------------------------------------------------
  if let Some(matches) = matches.subcommand_matches("serve") {

    let port: String = matches.get_one::<String>("port").cloned().unwrap_or("8081".to_string());
    let address = matches.get_one::<String>("address").cloned().unwrap_or("127.0.0.1".to_string());
    let full_address: String = format!("{}:{}",address,port);
    let mech_paths: Vec<String> = matches.get_many::<String>("mech_serve_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let stylesheet = matches.get_one::<String>("stylesheet").cloned().unwrap_or("include/style.css".to_string());
    let wasm_pkg = matches.get_one::<String>("wasm").cloned().unwrap_or("src/wasm/pkg".to_string());
   
    let mut server = MechServer::new(full_address, stylesheet.to_string(), wasm_pkg.to_string());
    server.init().await?;
    server.load_sources(&mech_paths)?;
    server.serve().await?;
    
  }
  // --------------------------------------------------------------------------
  // Format
  // --------------------------------------------------------------------------
  if let Some(matches) = matches.subcommand_matches("format") {
    let html_flag = matches.get_flag("html");
    let stylesheet_url = matches.get_one::<String>("stylesheet").cloned().unwrap_or("https://gitlab.com/mech-lang/mech/-/raw/v0.2-beta/include/style.css?ref_type=heads".to_string());
    let output_path = PathBuf::from(matches.get_one::<String>("output_path").cloned().unwrap_or(".".to_string()));

    let mech_paths: Vec<String> = matches.get_many::<String>("mech_format_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let mut mechfs = MechFileSystem::new();
    
    // open file or url. If it's a local file load it from disk, if it's a url fetch it from internet
    let stylesheet = if stylesheet_url.starts_with("http") {
      match reqwest::get(&stylesheet_url).await {
        Ok(response) => match response.text().await {
          Ok(text) => text,
          Err(err) => {
            println!("Error fetching stylesheet text: {:?}", err);
            //return Err(MechError::new(MechErrorKind::NetworkError));
            todo!()
          }
        },
        Err(err) => {
          println!("Error fetching stylesheet: {:?}", err);
          //return Err(MechError::new(MechErrorKind::NetworkError));
          todo!()
        }
      }
    } else {
      match fs::read_to_string(&stylesheet_url) {
        Ok(content) => content,
        Err(err) => {
          println!("Error reading stylesheet file: {:?}", err);
          //return Err(MechError::new(MechErrorKind::FileReadError));
          todo!()
        }
      }
    };

    mechfs.set_stylesheet(&stylesheet);
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
  let mut paths = if let Some(m) = matches.get_many::<String>("mech_paths") {
    m.map(|s| s.to_string()).collect()
  } else { repl_flag = true; vec![] };

  // Run the code
  let mut mechfs = MechFileSystem::new();
  for p in paths {
    mechfs.watch_source(&p)?;
  }

  /*let code = match mechfs.read_mech_files(&paths) {
    Ok(code) => code,
    Err(err) => {
      // treat the input args as a code instead of paths to files
      let code = paths.join(" ");
      vec![("shell".to_string(),MechSourceCode::String(code))]
    }
  };*/

  let result = run_mech_code(&mut intrp, &mechfs, tree_flag, debug_flag, time_flag); 
  
  let return_value = match &result {
    Ok(ref r) => {
      println!("{}", r.pretty_print());
      Ok(())
    }
    Err(ref err) => {
      Err(err.clone())
    }
  };

  if !repl_flag {
    return return_value;
  }

  let mut repl = MechRepl::from(intrp);
  
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
  let mut caught_inturrupts = Arc::new(Mutex::new(0));
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
  
  // --------------------------------------------------------------------------
  // REPL
  // --------------------------------------------------------------------------
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
              println!("{:?}", err);
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
          println!("{:?}", err);
        }
      }
    }
  }
  
  Ok(())
}
