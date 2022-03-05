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

lazy_static! {
  static ref CORE_MAP: Mutex<HashMap<SocketAddr, (String, SystemTime)>> = Mutex::new(HashMap::new());
}

// ## Mech Entry
#[tokio::main]
async fn main() -> Result<(), MechError> {

  let text_logo = r#"
  ┌─────────┐ ┌──────┐ ┌─┐ ┌──┐ ┌─┐   ┌─┐
  └───┐ ┌─┐ │ └──────┘ │ │ └┐ │ │ │   │ │
  ┌─┐ │ │ │ │ ┌──────┐ │ │  └─┘ │ └─┐ │ │
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
      .about("Execute all tests of a local package")
      .arg(Arg::with_name("mech_test_file_paths")
        .help("The files and folders to run.")
        .required(true)
        .multiple(true)))
    .subcommand(SubCommand::with_name("clean")
      .about("Remove the machines folder"))
    .subcommand(SubCommand::with_name("run")
      .about("Run a target folder or *.mec file")
      .arg(Arg::with_name("repl_mode")
        .short("r")
        .long("repl")
        .value_name("REPL")
        .help("Start a REPL")
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

    let index = warp::get()
                .and(warp::path::end())
                .and(warp::fs::dir("./notebook/"));

    let pkg = warp::path("pkg")
              .and(warp::fs::dir("./notebook/pkg"));

    let blocks = warp::path("blocks")
                .and(warp::addr::remote())
                .map(move |addr: Option<SocketAddr>| {
                  println!("{} Connection from {}", "[Mech Server]".bright_cyan(), addr.unwrap());
                  let code = read_mech_files(&mech_paths).unwrap();
                  // TODO Handle error situations
                  let miniblocks = compile_code(code).unwrap();
                  let serialized_miniblocks = bincode::serialize(&miniblocks).unwrap();
                  let compressed_miniblocks = compress_to_vec(&serialized_miniblocks,6);
                  encode(compressed_miniblocks)
                });

    let routes = index.or(pkg).or(blocks);

    println!("{} Awaiting connection at {}", "[Mech Server]".bright_cyan(), full_address);
    let socket_address: SocketAddr = full_address.parse().unwrap();
    warp::serve(routes).run(socket_address).await;

    println!("{} Closing server.", "[Mech Server]".bright_cyan());
    std::process::exit(0);

    None
  // ------------------------------------------------
  // TEST
  // ------------------------------------------------
  } else if let Some(matches) = matches.subcommand_matches("test") {
    /*
    println!("{}", "[Testing]".bright_green());
    let mut mech_paths: Vec<String> = matches.values_of("mech_test_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let mut passed_all_tests = true;
    mech_paths.push("https://gitlab.com/mech-lang/machines/mech/-/raw/main/src/test.mec".to_string());
    let code = read_mech_files(&mech_paths)?;
    let programs = compile_code(code);

    println!("{}", "[Running]".bright_green());
    let runner = ProgramRunner::new("Mech Test", 1000);
    let mech_client = runner.run();
    mech_client.send(RunLoopMessage::Code(MechCode::MiniPrograms(programs)));
    
    let mut tests_count = 0;
    let mut tests_passed = 0;
    let mut tests_failed = 0;
    
    let formatted_name = format!("\n[{}]", mech_client.name).bright_cyan();
    let thread_receiver = mech_client.incoming.clone();

    'receive_loop: loop {
      match thread_receiver.recv() {
        (Ok(ClientMessage::String(message))) => {
          println!("{} {}", formatted_name, message);
        },
        (Ok(ClientMessage::Table(table))) => {
          match table {
            Some(test_results) => {
              println!("{} Running {} tests...\n", formatted_name, test_results.rows);
              let mut failed_tests = vec![];
              for i in 1..=test_results.rows as usize {
                tests_count += 1;
                
                /*let test_name = match test_results.get_string(&TableIndex::Index(i),&TableIndex::Alias(*NAME)) {
                  Some((string,_)) => {
                    string.to_string()
                  }
                  _ => "".to_string()
                };*/
      
                //let test_result = test_results.get(&TableIndex::Index(i),&TableIndex::Alias(*RESULT)).unwrap();
                /*let test_result_string = match test_result.as_bool() {
                  Some(false) => {
                    passed_all_tests = false;
                    tests_failed += 1;
                    failed_tests.push(test_name.clone());
                    format!("{}", "failed".red())
      
                  },
                  Some(true) => {
                    tests_passed += 1;
                    format!("{}", "ok".green())
                  }
                  x => {
                    passed_all_tests = false;
                    tests_failed += 1;
                    failed_tests.push(test_name.clone());
                    format!("{}", "failed".red())
                  },
                };
                println!("\t{0: <30} {1: <5}", test_name, test_result_string);*/
              }

              if passed_all_tests {
                println!("\nTest result: {} | total {} | passed {} | failed {} | \n", "ok".green(), tests_count, tests_passed, tests_failed);
                std::process::exit(0);
              } else {
                println!("\nTest result: {} | total {} | passed {} | failed {} | \n", "failed".red(), tests_count, tests_passed, tests_failed);
                println!("\nFailed tests:\n");
                for failed_test in &failed_tests {
                  println!("\t{}", failed_test);
                }
                print!("\n");
                std::process::exit(failed_tests.len() as i32);
              }
            }
            None => println!("{} Table not found", formatted_name),
          }
          std::process::exit(0);
        },
        (Ok(ClientMessage::Transaction(txn))) => {
          println!("{} Transaction: {:?}", formatted_name, txn);
        },
        (Ok(ClientMessage::Done)) => {
          // Do nothing
        },
        Ok(ClientMessage::StepDone) => {
          mech_client.send(RunLoopMessage::GetTable(*MECH_TEST));
          //std::process::exit(0);
        },
        (Err(x)) => {
          println!("{} {}", "[Error]".bright_red(), x);
          std::process::exit(1);
        }
        q => {
          //println!("else: {:?}", q);
        },
      };
      io::stdout().flush().unwrap();
    }*/
    None
  // ------------------------------------------------
  // RUN
  // ------------------------------------------------
  } else if let Some(matches) = matches.subcommand_matches("run") {

    let mech_paths: Vec<String> = matches.values_of("mech_run_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let repl_flag = matches.is_present("repl_mode");    
    let debug_flag = matches.is_present("debug");    
    let input_arguments = matches.values_of("inargs").map_or(vec![], |inargs| inargs.collect());
    let out_tables = matches.values_of("out").map_or(vec![], |out| out.collect());
    let address: String = matches.value_of("address").unwrap_or("127.0.0.1").to_string();
    let port: String = matches.value_of("port").unwrap_or("0").to_string();
    let maestro_address: String = matches.value_of("maestro").unwrap_or("127.0.0.1:3235").to_string();
    let websocket_address: String = matches.value_of("websocket").unwrap_or("127.0.0.1:3236").to_string();

    let mut code: Vec<MechCode> = match read_mech_files(&mech_paths) {
      Ok(code) => code,
      Err(mech_error) => {
        println!("{}",format_errors(&vec![mech_error.kind]));
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
        println!("{}",format_errors(&vec![mech_error.kind]));
        std::process::exit(1);
      }
    };

    println!("{}", "[Running]".bright_green());

    let runner = ProgramRunner::new("Mech Runner", 1000);
    let mech_client = runner.run()?;
    mech_client.send(RunLoopMessage::Code(MechCode::MiniBlocks(blocks)));

    let formatted_name = format!("[{}]", mech_client.name).bright_cyan();
    let mech_client_name = mech_client.name.clone();
    let mech_client_channel = mech_client.outgoing.clone();
    let mech_client_channel_ws = mech_client.outgoing.clone();
    let mech_client_channel_heartbeat = mech_client.outgoing.clone();
    let mech_socket_address = mech_client.socket_address.clone();
    match mech_socket_address {
      Some(mech_socket_address) => {
        println!("{} Core socket started at: {}", formatted_name, mech_socket_address.clone());
        thread::Builder::new().name("Core socket".to_string()).spawn(move || {
          let formatted_name = format!("[{}]", mech_client_name).bright_cyan();
          // A socket bound to 3235 is the maestro. It will be the one other cores search for
          'socket_loop: loop {
            match UdpSocket::bind(maestro_address.clone()) {
              // The maestro core
              Ok(socket) => {
                println!("{} {} Socket started at: {}", formatted_name, "[Maestro]".truecolor(246,192,78), maestro_address);
                let mut buf = [0; 16_383];
                // Heartbeat thread periodically checks to see how long it's been since we've last heard from each remote core
                thread::Builder::new().name("Heartbeat".to_string()).spawn(move || {
                  loop {
                    thread::sleep(Duration::from_millis(500));
                    let now = SystemTime::now();
                    let mut core_map = CORE_MAP.lock().unwrap();
                    // If a core hasn't been heard from since 1 second ago, disconnect it.
                    for (_, (remote_core_address, _)) in core_map.drain_filter(|_k,(_, last_seen)| now.duration_since(*last_seen).unwrap().as_secs_f32() > 1.0) {
                      mech_client_channel_heartbeat.send(RunLoopMessage::RemoteCoreDisconnect(hash_str(&remote_core_address.to_string())));
                    }
                  }
                });
                // TCP socket thread for websocket connections
                thread::Builder::new().name("TCP Socket".to_string()).spawn(move || {
                  let server = Server::bind(websocket_address.clone()).unwrap();
                  println!("{} {} Websocket server started at: {}", formatted_name, "[Maestro]".truecolor(246,192,78), websocket_address);
                  for request in server.filter_map(Result::ok) {
                    let mut ws_stream = request.accept().unwrap();
                    let address = ws_stream.peer_addr().unwrap();
                    mech_client_channel_ws.send(RunLoopMessage::RemoteCoreConnect(MechSocket::WebSocket(ws_stream)));
                  }
                });

                // Loop to receive UDP messages from remote cores
                loop {
                  let (amt, src) = socket.recv_from(&mut buf).unwrap();
                  let now = SystemTime::now();
                  let message: Result<SocketMessage, bincode::Error> = bincode::deserialize(&buf);
                  match message {
                    // If a remote core connects, send a connection message back to it
                    Ok(SocketMessage::RemoteCoreConnect(remote_core_address)) => {
                      CORE_MAP.lock().unwrap().insert(src,(remote_core_address.clone(), SystemTime::now()));
                      mech_client_channel.send(RunLoopMessage::RemoteCoreConnect(MechSocket::UdpSocket(remote_core_address)));
                      let message = bincode::serialize(&SocketMessage::RemoteCoreConnect(mech_socket_address.clone())).unwrap();
                      let len = socket.send_to(&message, src.clone()).unwrap();
                    },
                    Ok(SocketMessage::Ping) => {
                      let mut core_map = CORE_MAP.lock().unwrap();
                      match core_map.get_mut(&src) {
                        Some((_, last_seen)) => {
                          *last_seen = now;
                        } 
                        None => (),
                      }
                      let message = bincode::serialize(&SocketMessage::Pong).unwrap();
                      let len = socket.send_to(&message, src).unwrap();
                    },
                    _ => (),
                  }
                }
              }
              // Maestro port is already bound, start a remote core
              Err(_) => {
                let socket = UdpSocket::bind(format!("{}:{}",address,port)).unwrap();
                let message = bincode::serialize(&SocketMessage::RemoteCoreConnect(mech_socket_address.clone().to_string())).unwrap();
                // Send a remote core message to the maestro
                let len = socket.send_to(&message, maestro_address.clone()).unwrap();
                let mut buf = [0; 16_383];
                loop {
                  let message = bincode::serialize(&SocketMessage::Ping).unwrap();
                  let len = socket.send_to(&message, maestro_address.clone()).unwrap();
                  match socket.recv_from(&mut buf) {
                    Ok((amt, src)) => {
                      let now = SystemTime::now();
                      if src.to_string() == maestro_address {
                        let message: Result<SocketMessage, bincode::Error> = bincode::deserialize(&buf);
                        match message {
                          Ok(SocketMessage::Pong) => {
                            thread::sleep(Duration::from_millis(500));
                            // Maestro is still alive
                          },
                          Ok(SocketMessage::RemoteCoreConnect(remote_core_address)) => {
                            CORE_MAP.lock().unwrap().insert(src,(remote_core_address.clone(), SystemTime::now()));
                            mech_client_channel.send(RunLoopMessage::RemoteCoreConnect(MechSocket::UdpSocket(remote_core_address)));
                          }
                          _ => (),
                        }
                      }
                    } 
                    Err(_) => {
                      println!("{} Maestro is dead.", formatted_name);
                      continue 'socket_loop;
                    }
                  }
                }
              }
            }
          }
        });
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
    'receive_loop: loop {
      match thread_receiver.recv() {
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
          println!("StepDone");
          if debug_flag{
            mech_client.send(RunLoopMessage::PrintDebug);
          }
          if out_tables.len() > 0 {
            for table_name in &out_tables {
              mech_client.send(RunLoopMessage::PrintTable(hash_str(table_name)));
            }
          }
          if repl_flag {
            break 'receive_loop;
          }
          //let output_id: u64 = hash_str("mech/output"); 
          //mech_client.send(RunLoopMessage::GetTable(output_id));
          //std::process::exit(0);
        },
        (Err(x)) => {
          println!("{} {}", "[Error]".bright_red(), x);
          io::stdout().flush().unwrap();
          std::process::exit(1);
        }
        q => {
          println!("else: {:?}", q);
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
        println!("{}",format_errors(&vec![mech_error.kind]));
        std::process::exit(1);
      }
    };
    let programs = compile_code(code);

    let output_name = match matches.value_of("output_name") {
      Some(name) => format!("{}.blx",name),
      None => "output.blx".to_string(),
    };

    let file = OpenOptions::new().write(true).create(true).open(&output_name).unwrap();
    let mut writer = BufWriter::new(file);
    
    let result = bincode::serialize(&programs).unwrap();
    if let Err(e) = writer.write_all(&result) {
      panic!("{} Failed to write core! {:?}", "[Error]".bright_red(), e);
    }
    writer.flush().unwrap();

    println!("{} Wrote {}", "[Finished]".bright_green(), output_name);
    std::process::exit(0);
    None
  } else if let Some(matches) = matches.subcommand_matches("clean") {
    std::fs::remove_dir_all("machines");
    std::process::exit(0);
    None
  } else {
    None
  };

    let help_message = r#"
Available commands are: 

help    - displays this message
quit    - quits this REPL
core    - prints info about the current mech core
runtime - prints info about the runtime attached to the current core
clear   - reset the current core
"#;

  let mut stdo = stdout();
  stdo.execute(terminal::Clear(terminal::ClearType::All));
  stdo.execute(cursor::MoveTo(0,0));
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
      let runner = ProgramRunner::new("Mech REPL", 1000);
      runner.run()?
    }
  };

  let mut skip_receive = false;

  //ClientHandler::new("Mech REPL", None, None, None, cores);
  let formatted_name = format!("\n[{}]", mech_client.name).bright_cyan();
  let thread_receiver = mech_client.incoming.clone();

  // Break out receiver into its own thread
  let thread = thread::Builder::new().name("Mech Receiving Thread".to_string()).spawn(move || {
    let mut q = 0;

    // Get all responses from the thread
    'receive_loop: loop {
      match thread_receiver.recv() {
        (Ok(ClientMessage::Pause)) => {
          println!("{} Paused", formatted_name);
        },
        (Ok(ClientMessage::Resume)) => {
          println!("{} Resumed", formatted_name);
        },
        (Ok(ClientMessage::Clear)) => {
          println!("{} Cleared", formatted_name);
        },
        (Ok(ClientMessage::NewBlocks(count))) => {
          println!("Compiled {} blocks.", count);
        },
        (Ok(ClientMessage::String(message))) => {
          println!("{} {}", formatted_name, message);
          print!("{}", ">: ".truecolor(246,192,78));
        },
        /*(Ok(ClientMessage::Table(table))) => {
          match table {
            Some(table) => {
              println!("{} ", formatted_name);

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
            None => println!("{} Table not found", formatted_name),
          }
        },*/
        (Ok(ClientMessage::Transaction(txn))) => {
          println!("{} Transaction: {:?}", formatted_name, txn);
        },
        (Ok(ClientMessage::Done)) => {
          print!("{}", ">: ".truecolor(246,192,78));
          // Do nothing
        },
        (Err(x)) => {
          println!("{} {}", "[Error]".bright_red(), x);
          break 'receive_loop;
        }
        q => {
          //println!("else: {:?}", q);
        },
      };
      io::stdout().flush().unwrap();
    }
  });


  'REPL: loop {
    
    io::stdout().flush().unwrap();
    // Print a prompt
    //print!("{}", ">: ".truecolor(246,192,78));
    io::stdout().flush().unwrap();
    let mut input = String::new();

    io::stdin().read_line(&mut input).unwrap();
    

    // Handle built in commands
    let parse = if input.trim() == "" {
      continue 'REPL;
    } else {
      parse_repl_command(input.trim())
    };
    
    match parse {
      Ok((_, command)) => {
        match command {
          ReplCommand::Help => {
            println!("{}",help_message);
          },
          ReplCommand::Quit => {
            println!("Quit");
            break 'REPL;
          },
          ReplCommand::Table(id) => {
            println!("Table {:?}", id);
            //mech_client.send(RunLoopMessage::Table(id));
          },
          ReplCommand::Clear => {
            println!("Clear");
            mech_client.send(RunLoopMessage::Clear);
          },
          ReplCommand::PrintCore(core_id) => {
            println!("PrintCore {:?}", core_id);
            mech_client.send(RunLoopMessage::PrintCore(core_id));
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
          ReplCommand::Error => {
            println!("Unknown command. Enter :help to see available commands.");
          },
          ReplCommand::Code(code) => {
            //println!("Code {:?}", code);
            mech_client.send(RunLoopMessage::Code(code));
          },
          _ => {
            println!("something else: {}", help_message);
          }
        }
      },
      _ => {
        
      }, 
    }
   
  }

  Ok(())
}