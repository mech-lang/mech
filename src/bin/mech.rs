#![feature(hash_extract_if)]
#![allow(warnings)]
extern crate tokio;
use mech::*;
use mech_core::*;
use mech_syntax::parser;
//use mech_syntax::analyzer::*;
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
use warp::http::header::{HeaderMap, HeaderValue};
use warp::Filter;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use base64::{encode, decode};
use chrono::Local;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<(), MechError> {
  /*panic::set_hook(Box::new(|panic_info| {
    if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
      println!("Mech Language Error: {}", s);
      // Check for underflow error message
      if s.contains("underflow") {
          println!("Underflow error occurred!");
      }
    } else {
        println!("Mech Language Error: Unknown panic");
    }
  }));*/
  
  let text_logo = r#"
  ┌─────────┐ ┌──────┐ ┌─┐ ┌──┐ ┌─┐   ┌─┐
  └───┐ ┌───┘ └──────┘ │ │ └┐ │ │ │   │ │
  ┌─┐ │ │ ┌─┐ ┌──────┐ │ │  └─┘ │ └─┐ │ │
  │ │ │ │ │ │ │ ┌────┘ │ │  ┌─┐ │ ┌─┘ │ │
  │ │ └─┘ │ │ │ └────┐ │ └──┘ │ │ │   │ │
  └─┘     └─┘ └──────┘ └──────┘ └─┘   └─┘"#.truecolor(246,192,78);

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
      .arg(Arg::new("address")
        .short('a')
        .long("address")
        .value_name("ADDRESS")
        .help("Sets the address of the server (127.0.0.1)")))
    .arg(Arg::new("tree")
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

  let mut intrp = Interpreter::new();

  // Serve
  // ----------------------------------------------------------------
  if let Some(matches) = matches.subcommand_matches("serve") {
    let server_badge = || {"[Mech Server]".truecolor(34, 204, 187)};
    ctrlc::set_handler(move || {
      println!("{} Server received shutdown signal. Process terminating.", server_badge());
      std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");

    let port: String = matches.get_one::<String>("port").cloned().unwrap_or("8081".to_string());
    let address = matches.get_one::<String>("address").cloned().unwrap_or("127.0.0.1".to_string());
    let full_address: String = format!("{}:{}",address,port);
    let mech_paths: Vec<String> = matches.get_many::<String>("mech_serve_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    
    // read index.html from disc
    let mech_html: String = fs::read_to_string("src/wasm/index.html").unwrap();
    let mech_wasm: Vec<u8> = fs::read("src/wasm/pkg/mech_wasm_bg.wasm").unwrap();
    let mech_js: Vec<u8> = fs::read("src/wasm/pkg/mech_wasm.js").unwrap();

    let code = match read_mech_files(&mech_paths) {
      Ok(code) => code,
      Err(err) => {
        println!("{:?}", err);
        vec![]
      }
    };

    // Serve the HTML file which includes the JS
    let mut headers = HeaderMap::new();
    headers.insert("content-type", HeaderValue::from_static("text/html"));
    let index = warp::get()
        .and(warp::path::end())
        .and(warp::filters::addr::remote()) // Capture remote address
        .map(move |remote: Option<SocketAddr>| {
            let date = Local::now();
            if let Some(addr) = remote {
              println!("{} {} - New connection from: {}", server_badge(), date.format("%Y-%m-%d %H:%M:%S"), addr);
            } else {
              println!("{} {} - New connection from unknown address", server_badge(), date.format("%Y-%m-%d %H:%M:%S"));
            }
            mech_html.clone()
        })
        .with(warp::reply::with::headers(headers));

    // Serve the JS file which includes the wasm
    let mut headers = HeaderMap::new();
    headers.insert("accept-ranges", HeaderValue::from_static("bytes"));
    headers.insert("content-type", HeaderValue::from_static("application/javascript"));
    let nb = warp::path!("pkg" / "mech_wasm.js")
              .map(move || {
                mech_js.clone()
              })
              .with(warp::reply::with::headers(headers));

    // Serve the wasm. This file is large so it's gzipped
    let mut headers = HeaderMap::new();
    headers.insert("accept-ranges", HeaderValue::from_static("bytes"));
    headers.insert("content-type", HeaderValue::from_static("application/wasm"));
    let pkg = warp::path!("pkg" / "mech_wasm_bg.wasm")
              .map(move || {
                mech_wasm.to_vec()
              })
              .with(warp::reply::with::headers(headers));
    
    let code = warp::path("code")
                .and(warp::addr::remote())
                .map(move |addr: Option<SocketAddr>| {
                  let (file,source) = &code[0];
                  let resp = if let MechSourceCode::String(s) = source {
                    s.clone()
                  } else {
                    "".to_string()
                  };
                  resp
                });    

    let routes = index.or(pkg).or(nb).or(code);

    println!("{} Awaiting connections at {}", server_badge(), full_address);
    let socket_address: SocketAddr = full_address.parse().unwrap();
    warp::serve(routes).run(socket_address).await;
    
    println!("{} Closing server.", server_badge());
    std::process::exit(0);
  }

  // Run
  // ----------------------------------------------------------------
  let mut paths = if let Some(m) = matches.get_many::<String>("mech_paths") {
    m.map(|s| s.to_string()).collect()
  } else { repl_flag = true; vec![] };

  match read_mech_files(&paths) {
    Ok(code) => {
      for c in code {
        match c {
          (file,MechSourceCode::String(s)) => {
            let now = Instant::now();
            let parse_result = parser::parse(&s.trim());
            let elapsed_time = now.elapsed();
            let parse_duration = elapsed_time.as_nanos() as f64;
            match parse_result {
              Ok(tree) => { 
                let now = Instant::now();
                let result = intrp.interpret(&tree);
                let elapsed_time = now.elapsed();
                let cycle_duration = elapsed_time.as_nanos() as f64;
                let result_str = match result {
                  Ok(r) => format!("{}", r.pretty_print()),
                  Err(err) => format!("{:?}", err),
                };
                println!("{}", result_str);
              },
              Err(err) => {
                if let MechErrorKind::ParserError(report, _) = err.kind {
                  parser::print_err_report(&s, &report);
                } else {
                  panic!("Unexpected error type");
                }
              }
            }
          }
          _ => todo!(),
        }
      }
    }
    Err(err) => println!("{:?}", err),
  }
  if !repl_flag {
    return Ok(());
  }
  
  #[cfg(windows)]
  control::set_virtual_terminal(true).unwrap();
  clc();
  let mut stdo = stdout();
  stdo.execute(Print(text_logo));
  stdo.execute(cursor::MoveToNextLine(1));
  println!("\n                {}                ",format!("v{}",VERSION).truecolor(246,192,78));
  println!("           {}           \n", "www.mech-lang.org");

  // Catch Ctrl-C a couple times before quitting
  let mut caught_inturrupts = Arc::new(Mutex::new(0));
  let mut ci = caught_inturrupts.clone();
  ctrlc::set_handler(move || {
    println!("[Ctrl+C]");
    let mut caught_inturrupts = ci.lock().unwrap();
    *caught_inturrupts += 1;
    if *caught_inturrupts >= 4 {
      println!("Okay cya!");
      std::process::exit(0);
    }
    println!("Type \":quit\" to terminate this REPL session.");
    print_prompt();
  }).expect("Error setting Ctrl-C handler");
  
  // REPL
  // ----------------------------------------------------------------
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
      // loop
      let repl_command = parse_repl_command(&input.as_str());

      match repl_command {
        Ok((_, ReplCommand::Help)) => println!("Mech REPL Commands: \n  :help, :h - Display this help message\n  :quit, :q, :exit - Quit the REPL\n  :symbols, :s - Display all symbols\n  :plan, :p - Display the plan\n  :whos, :w - Display all symbols\n  :clear - Clear the screen\n  :clc - Clear the screen\n  :load - Load a file\n  :save - Save a file\n  :step - Step through the plan"),
        Ok((_, ReplCommand::Quit)) => break 'REPL,
        Ok((_, ReplCommand::Symbols(name))) => println!("{}", intrp.symbols.borrow().pretty_print()),
        Ok((_, ReplCommand::Plan)) => println!("{}", pretty_print_plan(&intrp)),
        Ok((_, ReplCommand::Whos(name))) => println!("{}",whos(&intrp)),
        Ok((_, ReplCommand::Clear(name))) => {
          // Drop the old interpreter replace it with a new one
          intrp = Interpreter::new();
        }
        Ok((_, ReplCommand::Ls)) => {
          println!("{}",ls());
        }
        Ok((_, ReplCommand::Cd(path))) => {
          let path = PathBuf::from(path);
          env::set_current_dir(&path).unwrap();
        }
        Ok((_, ReplCommand::Clc)) => clc(),
        Ok((_, ReplCommand::Load(paths))) => {
          match read_mech_files(&paths) {
            Ok(code) => {
              for c in code {
                match c {
                  (file,MechSourceCode::String(s)) => {
                    let now = Instant::now();
                    let parse_result = parser::parse(&s.trim());
                    let elapsed_time = now.elapsed();
                    let parse_duration = elapsed_time.as_nanos() as f64;
                    match parse_result {
                      Ok(tree) => { 
                        let now = Instant::now();
                        let result = intrp.interpret(&tree);
                        let elapsed_time = now.elapsed();
                        let cycle_duration = elapsed_time.as_nanos() as f64;
                        let result_str = match result {
                          Ok(r) => format!("{}", r.pretty_print()),
                          Err(err) => format!("{:?}", err),
                        };
                        println!("{}", result_str);
                      },
                      Err(err) => {
                        if let MechErrorKind::ParserError(report, _) = err.kind {
                          parser::print_err_report(&s, &report);
                        } else {
                          panic!("Unexpected error type");
                        }
                      }
                    }
                  }
                  _ => todo!(),
                }
              }
            }
            Err(err) => println!("{:?}", err),
          }
        }
        Ok((_, ReplCommand::Step(count))) => {
          let n = match count {
            Some(n) => n,
            None => 1,
          };
          let plan_brrw = intrp.plan.borrow();
          let now = Instant::now();
          for i in 0..n {
            for fxn in plan_brrw.iter() {
              fxn.solve();
            }
          }
          let elapsed_time = now.elapsed();
          let cycle_duration = elapsed_time.as_nanos() as f64;
          println!("{:0.2?} ns", cycle_duration);
        }
        x => {
          let err = MechError{
            file: file!().to_string(),  
            tokens: vec![],
            msg: "".to_string(),
            id: line!(),
            kind: MechErrorKind::UnknownCommand(input.clone()),
          };
          println!("{:?}",x);
        }
      }
    } else if input.trim() == "" {
      continue;
    } else {
      // Treat as code
      match parser::parse(&input) {
        Ok(tree) => { 
          let now = Instant::now();
          let result = intrp.interpret(&tree);
          let elapsed_time = now.elapsed();
          let cycle_duration = elapsed_time.as_nanos() as f64;

          match result {
            Ok(r) => println!("{:?}\n{}", r.kind(), r.pretty_print()),
            Err(err) => println!("{:?}", err),
          }
          println!("{:0.2?} ns", cycle_duration);

        }
        Err(err) => {
          if let MechErrorKind::ParserError(report, _) = err.kind {
            parser::print_err_report(&input, &report);
          } else {
            panic!("Unexpected error type");
          }
        }
      }
    }
  }
  
  Ok(())
}
