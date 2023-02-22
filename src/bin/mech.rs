#![feature(hash_drain_filter)]
#![allow(warnings)]
// # Mech

/*
 Mech Server is a wrapper around the mech runtime. It provides interfaces for 
 controlling the runtime, sending it transactions, and responding to changes.
*/

// ## Prelude

extern crate core;
use std::io;

extern crate seahash;

extern crate miniz_oxide;

use miniz_oxide::inflate::decompress_to_vec;
use miniz_oxide::deflate::compress_to_vec;

extern crate base64;
use base64::{encode, decode};

extern crate clap;
use clap::{Arg, App, ArgMatches, SubCommand};

extern crate colored;
use colored::*;

extern crate mech;
use mech::*;
use mech_syntax::ast::Ast;
use mech_syntax::formatter::Formatter;
use mech_syntax::parser;

use std::thread::{self, JoinHandle};


extern crate reqwest;
use std::collections::HashMap;

extern crate bincode;
use std::io::{Write, BufReader, BufWriter, stdout};
use std::fs::{OpenOptions, File, canonicalize, create_dir};
use std::net::{SocketAddr, UdpSocket, TcpListener, TcpStream};
extern crate tokio;
use std::sync::Mutex;
extern crate websocket;
use websocket::sync::Server;
use websocket::OwnedMessage;
use std::sync::Arc;
use warp::http::header::{HeaderMap, HeaderValue};

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate serde;

//extern crate ws;
//use ws::{listen, Handler as WsHandler, Sender as WsSender, Result as WsResult, Message as WsMessage, Handshake, CloseCode, Error as WsError};
use std::time::{Duration, SystemTime};
use std::rc::Rc;
use std::cell::Cell;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate crossbeam_channel;
use crossbeam_channel::{Sender, Receiver};

extern crate warp;
use warp::Filter;

extern crate tui;
use tui::backend::CrosstermBackend;
use tui::Terminal;
use tui::widgets::{Widget, Block as TuiBlock, Borders};
use tui::layout::{Layout, Constraint, Direction};

use crossterm::{
  ExecutableCommand, QueueableCommand,
  terminal, cursor, style::Print,
};

lazy_static! {
  static ref MECH_TEST: u64 = hash_str("mech/test");
  static ref NAME: u64 = hash_str("name");
  static ref RESULT: u64 = hash_str("result");
}

// ## Mech Entry
#[tokio::main]
async fn main() -> Result<(), MechError> {

  let text_logo = r#"
  ┌─────────┐ ┌──────┐ ┌─┐ ┌──┐ ┌─┐   ┌─┐
  └───┐ ┌───┘ └──────┘ │ │ └┐ │ │ │   │ │
  ┌─┐ │ │ ┌─┐ ┌──────┐ │ │  └─┘ │ └─┐ │ │
  │ │ │ │ │ │ │ ┌────┘ │ │  ┌─┐ │ ┌─┘ │ │
  │ │ └─┘ │ │ │ └────┐ │ └──┘ │ │ │   │ │
  └─┘     └─┘ └──────┘ └──────┘ └─┘   └─┘"#.truecolor(246,192,78);

  #[cfg(windows)]
  control::set_virtual_terminal(true).unwrap();
  let version = "0.1.0";
  let matches = App::new("Mech")
    .version(version)
    .author("Corey Montella corey@mech-lang.org")
    .about(&*format!("{}", text_logo))
    .subcommand(SubCommand::with_name("serve")
      .about("Starts a Mech HTTP and websocket server")
      .arg(Arg::with_name("mech_serve_file_paths")
        .help("Source .mec and .blx files")
        .required(false)
        .multiple(true))
      .arg(Arg::with_name("port")
        .short("p")
        .long("port")
        .value_name("PORT")
        .help("Sets the port for the server (8081)")
        .takes_value(true))
      .arg(Arg::with_name("address")
        .short("a")
        .long("address")
        .value_name("ADDRESS")
        .help("Sets the address of the server (127.0.0.1)")
        .takes_value(true))  
      .arg(Arg::with_name("persist")
        .short("r")
        .long("persist")
        .value_name("PERSIST")
        .help("The path for the file to load from and persist changes (current working directory)")
        .takes_value(true)))
    .subcommand(SubCommand::with_name("test")
      .about("Run tests in a target folder or *.mec file.")
      .arg(Arg::with_name("mech_test_file_paths")
        .help("The files and folders to test.")
        .required(true)
        .multiple(true)))
    .subcommand(SubCommand::with_name("clean")
      .about("Remove the machines folder"))
    .subcommand(SubCommand::with_name("format")
      .about("Formats Mech source code according to a prescribed style.")
      .arg(Arg::with_name("output_name")
        .help("Output file name or directory")
        .short("o")
        .long("output")
        .value_name("OUTPUTNAME")
        .required(false)
        .takes_value(true))
      .arg(Arg::with_name("mech_format_file_paths")
        .help("The files and folders to format.")
        .required(true)
        .multiple(true))
      .arg(Arg::with_name("html")
        .short("h")
        .long("html")
        .value_name("HTML")
        .help("Format with HTML.")
        .required(false)
        .takes_value(false)))
    .subcommand(SubCommand::with_name("langserver")
      .about("Run a local mech language server")
      .arg(Arg::with_name("port")
        .short("p")
        .long("port")
        .value_name("PORT")
        .help("Sets the port for the server (default: 4041)")
        .takes_value(true)))
    .subcommand(SubCommand::with_name("run")
      .about("Run a target folder or *.mec file")
      .arg(Arg::with_name("repl_mode")
        .short("r")
        .long("repl")
        .value_name("REPL")
        .help("Start a REPL")
        .takes_value(false))
      .arg(Arg::with_name("timings")
        .short("t")
        .long("timings")
        .value_name("TIMINGS")
        .help("Displays transaction frequency in Hz.")
        .takes_value(false))
      .arg(Arg::with_name("debug")
        .short("d")
        .long("debug")
        .value_name("Debug")
        .help("Print debug info")
        .multiple(true)
        .required(false)
        .takes_value(false))
      .arg(Arg::with_name("out")
        .short("o")
        .long("out")
        .value_name("Out")
        .help("Specify output table(s)")
        .takes_value(true))
      .arg(Arg::with_name("inargs")
        .short("i")
        .long("inargs")
        .value_name("inargs")
        .help("Input arguments")
        .required(false)
        .multiple(true)
        .takes_value(true))   
      .arg(Arg::with_name("address")
        .short("a")
        .long("address")
        .value_name("ADDRESS")
        .help("Sets address of core socket (127.0.0.1)")
        .takes_value(true)) 
      .arg(Arg::with_name("port")
        .short("p")
        .long("port")
        .value_name("PORT")
        .help("Sets port of core socket (defaults to OS assigned port)")
        .takes_value(true)) 
      .arg(Arg::with_name("maestro")
        .short("m")
        .long("maestro")
        .value_name("MAESTRO")
        .help("Sets address of the maestro core (127.0.0.1:3235)")
        .takes_value(true))  
      .arg(Arg::with_name("websocket")
        .short("w")
        .long("websocket")
        .value_name("WEBSOCKET")
        .help("Sets the address of maestro websocket (127.0.0.1:3236)")
        .takes_value(true))  
      .arg(Arg::with_name("registry")
        .help("Location of the Mech machine registry.")
        .short("g")
        .long("registry")
        .value_name("REGISTRY")
        .takes_value(true))
      .arg(Arg::with_name("mech_run_file_paths")
        .help("The files and folders to run.")
        .required(true)
        .multiple(true)))
    .subcommand(SubCommand::with_name("build")
      .about("Build a target folder or *.mec file into a .blx file that can be loaded into a Mech runtime or compiled into an executable.")    
      .arg(Arg::with_name("output_name")
        .help("Output file name")
        .short("o")
        .long("output")
        .value_name("OUTPUTNAME")
        .required(false)
        .takes_value(true))
      .arg(Arg::with_name("mech_build_file_paths")
        .help("The files and folders to build.")
        .required(true)
        .multiple(true)))
    .get_matches();

  // ------------------------------------------------
  // SERVE
  // ------------------------------------------------
  let mech_client: Option<RunLoop> = if let Some(matches) = matches.subcommand_matches("serve") {

    let port = matches.value_of("port").unwrap_or("8081");
    let address = matches.value_of("address").unwrap_or("127.0.0.1");
    let full_address: String = format!("{}:{}",address,port);
    let mech_paths: Vec<String> = matches.values_of("mech_serve_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let persistence_path = matches.value_of("persistence").unwrap_or("");

    let mech_html = include_str!("../../wasm-notebook/index.html");
    let mech_wasm = include_bytes!("../../wasm-notebook/pkg/mech_wasm_notebook_bg.wasm");
    let mech_notebook = include_str!("../../wasm-notebook/pkg/mech_wasm_notebook.js");
    
    let mut headers = HeaderMap::new();
    headers.insert("content-type", HeaderValue::from_static("text/html"));
    let index = warp::get()
                .and(warp::path::end())
                .map(move || {
                  mech_html
                })
                .with(warp::reply::with::headers(headers));

    let mut headers = HeaderMap::new();
    headers.insert("accept-ranges", HeaderValue::from_static("bytes"));
    headers.insert("content-type", HeaderValue::from_static("application/javascript"));
    let nb = warp::path!("pkg" / "mech_notebook.js")
              .map(move || {
                mech_notebook
              })
              .with(warp::reply::with::headers(headers));

    let mut headers = HeaderMap::new();
    headers.insert("accept-ranges", HeaderValue::from_static("bytes"));
    headers.insert("content-type", HeaderValue::from_static("application/wasm"));
    let pkg = warp::path!("pkg" / "mech_wasm_notebook_bg.wasm")
              .map(move || {
                mech_wasm.to_vec()
              })
              .with(warp::reply::with::headers(headers));
    
    let blocks = warp::path("blocks")
                .and(warp::addr::remote())
                .map(move |addr: Option<SocketAddr>| {
                  println!("{} Connection from {}", "[Mech Server]".truecolor(34,204,187), addr.unwrap());
                  let code = read_mech_files(&mech_paths).unwrap();
                  // TODO Handle error situations
                  let miniblocks = compile_code(code).unwrap();
                  let serialized_miniblocks = bincode::serialize(&miniblocks).unwrap();
                  let compressed_miniblocks = compress_to_vec(&serialized_miniblocks,6);
                  encode(compressed_miniblocks)
                });          

    let routes = index.or(pkg).or(nb).or(blocks);

    println!("{} Awaiting connection at {}", "[Mech Server]".truecolor(34,204,187), full_address);
    let socket_address: SocketAddr = full_address.parse().unwrap();
    warp::serve(routes).run(socket_address).await;

    println!("{} Closing server.", "[Mech Server]".truecolor(34,204,187));
    std::process::exit(0);

    None
  // ------------------------------------------------
  // TEST
  // ------------------------------------------------
  } else if let Some(matches) = matches.subcommand_matches("test") {
    
    println!("{}", "[Testing]".truecolor(153,221,85));
    let mut mech_paths: Vec<String> = matches.values_of("mech_test_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let mut passed_all_tests = true;
    mech_paths.push("https://gitlab.com/mech-lang/machines/mech/-/raw/v0.1-beta/src/test.mec".to_string());
    let code = read_mech_files(&mech_paths)?;

    let blocks = match compile_code(code) {
      Ok(blocks) => blocks,
      Err(mech_error) => {
        println!("{}",format_errors(&vec![mech_error]));
        std::process::exit(1);
      }
    };

    println!("{}", "[Running]".truecolor(153,221,85));
    let runner = ProgramRunner::new("Mech Test");
    let mech_client = runner.run()?;
    mech_client.send(RunLoopMessage::Code((1,MechCode::MiniBlocks(blocks))));

    let mut tests_count = 0;
    let mut tests_passed = 0;
    let mut tests_failed = 0;
    
    let formatted_name = format!("\n[{}]", mech_client.name).truecolor(34,204,187);
    let thread_receiver = mech_client.incoming.clone();

    let mut to_exit = false;
    let mut exit_code = 0;

    'receive_loop: loop {
      match thread_receiver.recv() {
        (Ok(ClientMessage::Ready)) => {
          println!("{} Ready", formatted_name);
        },
        (Ok(ClientMessage::String(message))) => {
          println!("{} {}", formatted_name, message);
        },
        (Ok(ClientMessage::Transaction(txn))) => {
          println!("{} Transaction: {:?}", formatted_name, txn);
        },
        (Ok(ClientMessage::Timing(_))) => {
          // Do nothing
        },
        (Ok(ClientMessage::Done)) => {
          if to_exit {
            io::stdout().flush().unwrap();
            std::process::exit(exit_code);
          }
        },
        (Ok(ClientMessage::Exit(this_code))) => {
          to_exit = true;
          exit_code = this_code;
        }
        Ok(ClientMessage::StepDone) => {
          //mech_client.send(RunLoopMessage::GetTable(*MECH_TEST));
          //std::process::exit(0);
        },
        Ok(ClientMessage::Error(err)) => {
          println!("{} {} An Error Has Occurred: {:?}", formatted_name, "[Error]".truecolor(170,51,85), err);
        }
        (Err(x)) => {
          println!("{} {}", "[Error]".truecolor(170,51,85), x);
          std::process::exit(1);
        }
        q => {
          //println!("*else: {:?}", q);
        },
      };
      io::stdout().flush().unwrap();
    }
    None
  // ------------------------------------------------
  // RUN
  // ------------------------------------------------
  } else if let Some(matches) = matches.subcommand_matches("run") {
    let mech_paths: Vec<String> = matches.values_of("mech_run_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let repl_flag = matches.is_present("repl_mode");    
    let debug_flag = matches.is_present("debug");    
    let timings_flag = matches.is_present("timings");    
    let machine_registry = matches.value_of("registry").unwrap_or("https://gitlab.com/mech-lang/machines/mech/-/raw/v0.1-beta/src/registry.mec").to_string();
    let input_arguments = matches.values_of("inargs").map_or(vec![], |inargs| inargs.collect());
    let out_tables = matches.values_of("out").map_or(vec![], |out| out.collect());
    let address: String = matches.value_of("address").unwrap_or("127.0.0.1").to_string();
    let port: String = matches.value_of("port").unwrap_or("0").to_string();
    let maestro_address: String = matches.value_of("maestro").unwrap_or("127.0.0.1:3235").to_string();
    let websocket_address: String = matches.value_of("websocket").unwrap_or("127.0.0.1:3236").to_string();

    let mut code: Vec<MechCode> = match read_mech_files(&mech_paths) {
      Ok(code) => code,
      Err(mech_error) => {
        println!("{}",format_errors(&vec![mech_error]));
        std::process::exit(1);
      }
    };

    /*if input_arguments.len() > 0 {
      let arg_string: String = input_arguments.iter().fold("".to_string(), |acc, arg| format!("{}\"{}\";",acc,arg));
      let inargs_code = format!("#system/input-arguments += [{}]", arg_string);
      code.push(MechCode::String(inargs_code));
    }*/
    
    let blocks = match compile_code(code) {
      Ok(blocks) => blocks,
      Err(mech_error) => {
        println!("{}",format_errors(&vec![mech_error]));
        std::process::exit(1);
      }
    };

    println!("{}", "[Running]".truecolor(153,221,85));

    let mut runner = ProgramRunner::new("Run");
    runner.registry = machine_registry;
    let mech_client = runner.run()?;
    mech_client.send(RunLoopMessage::Code((1,MechCode::MiniBlocks(blocks))));

    let formatted_name = format!("[{}]", mech_client.name).truecolor(34,204,187).to_string();
    let mech_client_name = mech_client.name.clone();
    let mech_client_channel = mech_client.outgoing.clone();   

    let mech_socket_address = mech_client.socket_address.clone();
    let mut core_socket_thread;
    let formatted_address = format!("{}:{}",address,port);
    match mech_socket_address {
      Some(mech_socket_address) => {
        core_socket_thread = start_maestro(
          mech_socket_address, 
          formatted_address, 
          maestro_address, 
          websocket_address, 
          mech_client_channel);
      }
      None => (),
    };

    //ClientHandler::new("Mech REPL", None, None, None, cores);
    let thread_receiver = mech_client.incoming.clone();
    
    // Some state variables to control receive loop
    let mut skip_receive = false;
    let mut to_exit = false;
    let mut exit_code = 0;

    // Get all responses from the thread
    'run_receive_loop: loop {
      match thread_receiver.recv() {
        (Ok(ClientMessage::Ready)) => {
          println!("{} {}", formatted_name, "[Ready]".truecolor(153,221,85));
        },
        (Ok(ClientMessage::Timing(freqeuncy))) => {
          if timings_flag {
            println!("{} Txn took: {:.2?}Hz", formatted_name, freqeuncy);
          }
        },
        (Ok(ClientMessage::String(message))) => {
          println!("{} {}", formatted_name, message);
        },
        (Ok(ClientMessage::Error(error))) => {
          let formatted_errors = format_errors(&vec![error]);
          println!("{}\n{}", formatted_name, formatted_errors);
        },
        (Ok(ClientMessage::Transaction(txn))) => {
          println!("{} Transaction: {:?}", formatted_name, txn);
        },
        (Ok(ClientMessage::Done)) => {
          if to_exit {
            io::stdout().flush().unwrap();
            std::process::exit(exit_code);
          }
        },
        (Ok(ClientMessage::Exit(this_code))) => {
          to_exit = true;
          exit_code = this_code;
        }
        Ok(ClientMessage::StepDone) => {
          if debug_flag{
            mech_client.send(RunLoopMessage::PrintCore(Some(1)));
          }
          if out_tables.len() > 0 {
            for table_name in &out_tables {
              mech_client.send(RunLoopMessage::PrintTable(hash_str(table_name)));
            }
          }
          if repl_flag {
            break 'run_receive_loop;
          }
          //let output_id: u64 = hash_str("mech/output"); 
          //mech_client.send(RunLoopMessage::GetTable(output_id));
          //std::process::exit(0);
        },
        (Err(x)) => {
          println!("{} {}", "[Error]".truecolor(170,51,85), x);
          io::stdout().flush().unwrap();
          std::process::exit(1);
        }
        q => {
          //println!("else: {:?}", q);
        },
      };
      io::stdout().flush().unwrap();
    }
    Some(mech_client)
  // ------------------------------------------------
  // BUILD a .blx file from .mec and other .blx files
  // ------------------------------------------------
  } else if let Some(matches) = matches.subcommand_matches("build") {
    let mech_paths: Vec<String> = matches.values_of("mech_build_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let mut code: Vec<MechCode> = match read_mech_files(&mech_paths) {
      Ok(code) => code,
      Err(mech_error) => {
        println!("{}",format_errors(&vec![mech_error]));
        std::process::exit(1);
      }
    };

    match compile_code(code) {
      Ok(sections) => {
        let output_name = match matches.value_of("output_name") {
          Some(name) => format!("{}.blx",name),
          None => "output.blx".to_string(),
        };
    
        let file = OpenOptions::new().write(true).create(true).open(&output_name).unwrap();
        let mut writer = BufWriter::new(file);
      
        let result = bincode::serialize(&sections).unwrap();
    
        if let Err(e) = writer.write_all(&result) {
          panic!("{} Failed to write core(s)! {:?}", "[Error]".truecolor(170,51,85), e);
          std::process::exit(1);
        }
        writer.flush().unwrap();
    
        println!("{} Wrote {}", "[Finished]".truecolor(153,221,85), output_name);
        std::process::exit(0);
      }
      Err(mech_error) => {
        println!("{}",format_errors(&vec![mech_error]));
        std::process::exit(1);
      }
    }
    None
  // ---------------------------------------------------
  // FORMAT standardize formatting of mech source files
  // ---------------------------------------------------
  } else if let Some(matches) = matches.subcommand_matches("format") {
    let html = matches.is_present("html");    
    let mech_paths: Vec<String> = matches.values_of("mech_format_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let mut code: Vec<MechCode> = match read_mech_files(&mech_paths) {
      Ok(code) => code,
      Err(mech_error) => {
        println!("{}",format_errors(&vec![mech_error]));
        std::process::exit(1);
      }
    };

    let mut source_trees = vec![];

    for c in code {
      match c {
        MechCode::String(source) => {
          let parse_tree = parser::parse(&source)?;
          let mut ast = Ast::new();
          ast.build_syntax_tree(&parse_tree);
          source_trees.push(ast.syntax_tree.clone());
        }
        _ => (), 
      }
    }

    let formatted_source = source_trees.iter().map(|t| {
      let mut f = Formatter::new();
      if html {
        f.format_html(&t)
      } else {
        f.format(&t)
      }
    }).collect::<Vec<String>>();
  
    for f in formatted_source {
      if html {
        let mut file = File::create("index.html")?;
        file.write_all(f.as_bytes())?;
      } else {
        let mut file = File::create("index.mec")?;
        file.write_all(f.as_bytes())?;
      }
    }
    std::process::exit(0);

    None    
  // ------------------------------------------------
  //  Clean
  // ------------------------------------------------
  } else if let Some(matches) = matches.subcommand_matches("clean") {
    std::fs::remove_dir_all("machines");
    std::process::exit(0);
    None
  // ------------------------------------------------
  //  Run language server
  // ------------------------------------------------
  } else if let Some(matches) = matches.subcommand_matches("langserver") {
    let address = "localhost".to_owned();
    let port = matches.value_of("port").unwrap_or("4041").to_string();
    println!("{} Starting language server at {}:{}", "[INFO]".truecolor(34,204,187), address, port);
    mech_syntax::langserver::run_langserver(&address, &port).await?;
    std::process::exit(0);
    None
  // ------------------------------------------------
  //  Not matched
  // ------------------------------------------------
  } else {
    None
  };

  let help_message = r#"
Available commands are: 

core    - switch active context to a provided core
debug   - print debug info about the active context
help    - displays this message
info    - print diagnostic info about the REPL environment
load    - load a .mec or .blx file (or mech project folder)
new     - start a new core, switch active context to that core
quit    - quits this REPL
reset   - reset the actibe context
save    - save the state of a core to disk as a .blx file"#;

  let mut stdo = stdout();
  stdo.execute(terminal::Clear(terminal::ClearType::All));
  stdo.execute(cursor::MoveTo(0,0));
  /*
  magenta     truecolor(136,17,119)
  red         truecolor(170,51,85)
  pink        truecolor(204,102,102)
  orange      truecolor(238,153,68)
  yellow      truecolor(238,221,0)
  light green truecolor(153,221,85)
  dark green  truecolor(68,221,136)
  cyan        truecolor(34,204,187)
  light blue  truecolor(0,187,204)
  dark blue   truecolor(0,153,204)
  indigo      truecolor(51,102,187)
  violet      truecolor(102,51,153)
  */

  stdo.execute(Print(text_logo));
  stdo.execute(cursor::MoveToNextLine(1));
  
  println!(" {}",  "╔═══════════════════════════════════════╗".bright_black());
  println!(" {}                 {}                {}", "║".bright_black(), format!("v{}",version).truecolor(246,192,78), "║".bright_black());
  println!(" {}           {}           {}", "║".bright_black(), "www.mech-lang.org", "║".bright_black());
  println!(" {}\n",  "╚═══════════════════════════════════════╝".bright_black());

  println!("Prepend commands with a colon. Enter :help to see a full list of commands. Enter :quit to quit.\n");
  stdo.flush();


  // Start a new mech client if we didn't get one from another command
  let mech_client = match mech_client {
    Some(mech_client) => mech_client,
    None => {
      let runner = ProgramRunner::new("REPL");
      runner.run()?
    }
  };

  let mut skip_receive = false;

  //ClientHandler::new("Mech REPL", None, None, None, cores);
  let formatted_name1 = format!("\n[{}]", mech_client.name).truecolor(34,204,187);
  let formatted_name2 = formatted_name1.clone();
  let thread_receiver = mech_client.incoming.clone();

  // Break out receiver into its own thread
  let thread = thread::Builder::new().name("Mech Receiving Thread".to_string()).spawn(move || {
    let mut q = 0;
    // Get all responses from the thread
    'repl_receive_loop: loop {
      match thread_receiver.recv() {
        (Ok(ClientMessage::Pause)) => {
          println!("{} Paused", formatted_name1);
        },
        (Ok(ClientMessage::Resume)) => {
          println!("{} Resumed", formatted_name1);
        },
        (Ok(ClientMessage::Reset)) => {
          //println!("{} Reset", formatted_name1);
        },
        (Ok(ClientMessage::NewBlocks(count))) => {
          println!("Compiled {} blocks.", count);
        },
        (Ok(ClientMessage::String(message))) => {
          println!("{} {}", formatted_name1, message);
          print!("{}", ">: ".truecolor(246,192,78));
        },
        /*(Ok(ClientMessage::Table(table))) => {
          match table {
            Some(table) => {
              println!("{} ", formatted_name1);

              fn print_table(x: u16, y: u16, table: Table) {
                let mut stdout = stdout();

                let formatted_table = format!("{:?}",table);
                let mut lines = 0;

                //stdout.queue(cursor::MoveTo(x,y + lines as u16)).unwrap();
                for s in formatted_table.chars() {
                  if s == '\n' {
                    lines += 1;
                    //stdout.queue(cursor::MoveTo(x,y + lines as u16)).unwrap();
                    //continue;
                  }
                  stdout.queue(Print(s));
                }
                //stdout.queue(cursor::MoveTo(0,y + lines as u16)).unwrap();
                stdout.flush();
              }

              print_table(10,q,table);
              q += 10;

              print!("{}", ">: ".truecolor(246,192,78));
            }
            None => println!("{} Table not found", formatted_name1),
          }
        },*/
        (Ok(ClientMessage::Transaction(txn))) => {
          println!("{} Transaction: {:?}", formatted_name1, txn);
        },
        (Ok(ClientMessage::Done)) => {
          //print!("{}", ">: ".truecolor(246,192,78));
          // Do nothing
        },
        (Err(x)) => {
          println!("{} {}", "[Error]".truecolor(170,51,85), x);
          break 'repl_receive_loop;
        }
        q => {
          //println!("else: {:?}", q);
        },
      };
      io::stdout().flush().unwrap();
    }
  });

  let mut current_core: u64 = 1;

  'REPL: loop {

    io::stdout().flush().unwrap();
    // Print a prompt 
    // 4, 8, 15, 16, 23, 42
    print!("{}", ">: ".truecolor(246,192,78));
    io::stdout().flush().unwrap();
    let mut input = String::new();

    io::stdin().read_line(&mut input).unwrap();
    

    // Handle built in commands
    let parse = if input.trim() == "" {
      continue 'REPL;
    } else {
      parse(input.trim())
    };
    
    match parse {
      Ok(command) => {
        match command {
          ReplCommand::Help => {
            println!("{}",help_message);
          },
          ReplCommand::Quit => {
            println!("{} Bye!", formatted_name2);
            break 'REPL;
          },
          ReplCommand::Save => {
            println!("{} Saving Core {}", formatted_name2, current_core);
            mech_client.send(RunLoopMessage::DumpCore(current_core));
          },
          ReplCommand::Code(code) => {
            for c in code {
              mech_client.send(RunLoopMessage::Code((current_core,c)));
            }
          },
          ReplCommand::Reset => {
            mech_client.send(RunLoopMessage::Reset(current_core));
          },
          ReplCommand::NewCore => {
            mech_client.send(RunLoopMessage::NewCore);
          },
          ReplCommand::Core(core_id) => {
            println!("{} Switched to Core {}", formatted_name2, core_id);
            current_core = core_id;
          },
          ReplCommand::Stop => {
            println!("Stop");
          },          
          ReplCommand::Table(table_id) => {
            println!("Table");
          },
          ReplCommand::Info => {
            println!("{} Active context: Core {}", formatted_name2, current_core);
            mech_client.send(RunLoopMessage::PrintInfo);
          },
          ReplCommand::Debug => {
            mech_client.send(RunLoopMessage::PrintCore(Some(current_core)));
          },
          ReplCommand::Pause => {
            println!("Pause");
            mech_client.send(RunLoopMessage::Pause);
          },
          ReplCommand::Resume => {
            println!("Resume");
            mech_client.send(RunLoopMessage::Resume);
          },
          ReplCommand::Empty => {
            println!("Empty");
          },
          ReplCommand::Error(err) => {
            println!("{:?}", err);
          },
        }
      },
      Err(err) => {
        println!("{}",format_errors(&vec![err]));
      }, 
    }
  }

  Ok(())
}
