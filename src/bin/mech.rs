// # Mech

/*
 Mech Server is a wrapper around the mech runtime. It provides interfaces for 
 controlling the runtime, sending it transactions, and responding to changes.
*/

// ## Prelude

extern crate core;
use std::path::{Path, PathBuf};
use std::io;
use std::io::prelude::*;

extern crate clap;
use clap::{Arg, App, ArgMatches, SubCommand};

extern crate colored;
use colored::*;

extern crate mech;
use mech::{
  Core, 
  MiniBlock, 
  Block, 
  Constraint, 
  Compiler, 
  Table, 
  Value, 
  ParserNode, 
  Hasher, 
  Program, 
  ErrorType, 
  ProgramRunner, 
  RunLoop, 
  RunLoopMessage, 
  ClientMessage, 
  Parser,
  MechCode,
};
use mech::QuantityMath;

use std::thread::{self, JoinHandle};

#[macro_use]
extern crate nom;
use nom::{
  IResult,
  branch::alt,
  sequence::tuple,
  combinator::opt,
  error::{context, convert_error, ErrorKind, ParseError, VerboseError},
  multi::{many1, many0},
  bytes::complete::{tag},
  character::complete::{alphanumeric1, alpha1, digit1, space0, space1},
};

extern crate reqwest;
use std::collections::HashMap;

extern crate bincode;
use std::io::{Write, BufReader, BufWriter};
use std::fs::{OpenOptions, File, canonicalize, create_dir};

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate serde;

#[macro_use]
extern crate actix_web;
extern crate actix_rt;
extern crate actix_files;
use actix_web::{get, web, App as ActixApp, HttpServer, HttpResponse, Responder};
use actix_session::{CookieSession, Session};
use ahash::AHasher;

#[macro_use]
extern crate crossbeam_channel;
use crossbeam_channel::{Sender, Receiver};

#[derive(Debug, Clone)]
pub enum ReplCommand {
  Help,
  Quit,
  Pause,
  Resume,
  Stop,
  PrintCore(Option<u64>),
  PrintRuntime,
  Clear,
  Table(u64),
  Code(String),
  EchoCode(String),
  ParsedCode(ParserNode),
  Empty,
  Error,
}

async fn read_mech_files(mech_paths: Vec<&str>) -> Result<Vec<MechCode>, Box<dyn std::error::Error>> {

  let mut code: Vec<MechCode> = Vec::new();

  for path_str in mech_paths {
    let path = Path::new(path_str);
    // Compile a .mec file on the web
    if path.to_str().unwrap().starts_with("https") {
      println!("{} {}", "[Building]".bright_green(), path.display());
      let program = reqwest::get(path.to_str().unwrap()).await?.text().await?;
      code.push(MechCode::String(program));
    } else {
      // Compile a directory of mech files
      if path.is_dir() {
        for entry in path.read_dir().expect("read_dir call failed") {
          if let Ok(entry) = entry {
            match (entry.path().to_str(), entry.path().extension())  {
              (Some(name), Some(extension)) => {
                match extension.to_str() {
                  Some("blx") => {
                    println!("{} {}", "[Building]".bright_green(), name);
                    let file = File::open(name)?;
                    let mut reader = BufReader::new(file);
                    let miniblocks: Vec<MiniBlock> = bincode::deserialize_from(&mut reader)?;
                    code.push(MechCode::MiniBlock(miniblocks));
                  }
                  Some("mec") => {
                    println!("{} {}", "[Building]".bright_green(), name);
                    let mut f = File::open(name)?;
                    let mut buffer = String::new();
                    f.read_to_string(&mut buffer);
                    code.push(MechCode::String(buffer));
                  }
                  _ => (),
                }
              },
              _ => (),
            }
          }
        }
      } else if path.is_file() {
        // Compile a single file
        match (path.to_str(), path.extension())  {
          (Some(name), Some(extension)) => {
            match extension.to_str() {
              Some("blx") => {
                println!("{} {}", "[Building]".bright_green(), name);
                let file = File::open(name)?;
                let mut reader = BufReader::new(file);
                let miniblocks: Vec<MiniBlock> = bincode::deserialize_from(&mut reader)?;
                code.push(MechCode::MiniBlock(miniblocks));
              }
              Some("mec") => {
                println!("{} {}", "[Building]".bright_green(), name);
                let mut f = File::open(name)?;
                let mut buffer = String::new();
                f.read_to_string(&mut buffer);
                code.push(MechCode::String(buffer));
              }
              _ => (),
            }
          },
          _ => (),
        }
      }
    };
  }
  Ok(code)
}

fn compile_code(code: Vec<MechCode>) -> Vec<Block> {
  let mut compiler = Compiler::new();
  for c in code {
    match c {
      MechCode::String(c) => {compiler.compile_string(c);},
      MechCode::MiniBlock(c) => {
        let mut blocks: Vec<Block> = Vec::new();
        for miniblock in c {
          let mut block = Block::new();
          for constraint in miniblock.constraints {
            block.add_constraints(constraint);
          }
          blocks.push(block);
        }
        compiler.blocks.append(&mut blocks);
      },
    }
  }
  compiler.blocks
}

// ## Mech Entry
#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  #[cfg(windows)]
  control::set_virtual_terminal(true).unwrap();
  let version = "0.0.4";
  let matches = App::new("Mech")
    .version(version)
    .author("Corey Montella corey@mech-lang.org")
    .about("Mech's compiler and REPL. Also contains various other helpful tools! Default values for options are in parentheses.")
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
        .help("Sets the port for the Mech websocket server (3012)")
        .takes_value(true))
      .arg(Arg::with_name("http-port")
        .short("t")
        .long("http-port")
        .value_name("HTTPPORT")
        .help("Sets the port for the HTTP server (8081)")
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
    .subcommand(SubCommand::with_name("run")
      .about("Run a target folder or *.mec file")
      .arg(Arg::with_name("repl_mode")
        .short("r")
        .long("repl")
        .value_name("REPL")
        .help("Start a REPL")
        .takes_value(false))
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

  let core: Option<Core> = if let Some(matches) = matches.subcommand_matches("serve") {

    let wport = matches.value_of("port").unwrap_or("3012");
    let hport = matches.value_of("http-port").unwrap_or("8081");
    let address = matches.value_of("address").unwrap_or("127.0.0.1");
    let http_address = format!("{}:{}",address,hport);
    let websocket_address = format!("{}:{}",address,wport);
    let mech_paths = matches.values_of("mech_serve_file_paths").map_or(vec![], |files| files.collect());
    let persistence_path = matches.value_of("persistence").unwrap_or("");

    // Spin up a mech core and compiler
    let mut core = Core::new(1000,1000);

    let code = read_mech_files(mech_paths).await?;
    let blocks = compile_code(code.clone());

    let mut miniblocks: Vec<MiniBlock> = Vec::new();
    for block in blocks {
      let mut miniblock = MiniBlock::new();
      miniblock.constraints = block.constraints.clone();
      miniblocks.push(miniblock);
    }
    let serialized_miniblocks: Vec<u8> = bincode::serialize(&miniblocks).unwrap();

    let mut runner = ProgramRunner::new("Mech REPL", 1500000);
    let mech_client = runner.run();
    for c in code {
      mech_client.send(RunLoopMessage::Code((0,c)));
    }
    loop {
      match mech_client.receive() {
        Ok(ClientMessage::Done) => {
          if mech_client.is_empty() {
            break;
          }
        }
        x => println!("{:?}", x),
      }
    }

    async fn index(session: Session, req: web::HttpRequest, info: web::Path<(String)>, data: web::Data<(Sender<RunLoopMessage>,Receiver<ClientMessage>, Vec<u8>)>) -> impl Responder {
      println!("Serving");
      use core::hash::Hasher;
      //println!("Connection Info {:?}", req.connection_info());
      //println!("Head {:?}", req.head());
      let mut hasher = AHasher::new_with_keys(123, 456);
      hasher.write(format!("{:?}", req.head()).as_bytes());
      let mut id: u64 = hasher.finish();
      if let Some(uid) = session.get::<u64>("mech-user/id").unwrap() {
        println!("Mech user: {:x}", uid);
        id = uid
      } else {
        session.set("mech-user/id", id);
      }


      /*
      let (sender, receiver) = data.get_ref();
      loop {
        match receiver.recv() {
          Ok(ClientMessage::Done) => {
            if receiver.is_empty() {
              break;
            }
            
          }
          x => println!("{:?}", x),
        }
      }

      let code = format!("#ans = #{}", info);
      sender.send(RunLoopMessage::EchoCode(code));
      let mut message = String::new();
      loop {
        match receiver.recv() {
          Ok(ClientMessage::Done) => {
            if receiver.is_empty() {
              break;
            }
          }
          Err(x) => {
            println!("{:?}",x);
            break;
          }
          Ok(ClientMessage::Table(table)) => {
            message = format!("{:?}", table);
          }
          x => println!("{:?}", x),
        }
      }*/
        
      //message
      format!("{:x}", id)
    }

    async fn serve_blocks(data: web::Data<(Sender<RunLoopMessage>,Receiver<ClientMessage>,Vec<u8>)>) -> impl Responder {
      let (sender, receiver, miniblocks) = data.get_ref();
      format!("{{\"blocks\": {:?} }}", miniblocks)
    }

    println!("{} Awaiting connection at {}", "[Mech Server]".bright_cyan(), http_address);
    let data = web::Data::new((mech_client.outgoing.clone(), mech_client.incoming.clone(), serialized_miniblocks.clone()));
    HttpServer::new(move || {
        ActixApp::new()
        .app_data(data.clone())
        .wrap(CookieSession::signed(&[0; 32]).secure(false))
        .service(web::resource("/table/{query}")
          .route(web::get().to(index)))
        .service(web::resource("/blocks")
          .route(web::get().to(serve_blocks)))
        .service(
          actix_files::Files::new("/", "./notebook/").index_file("index.html"),
        )
      })
      .bind(http_address)?
      .run()
      .await?;
    println!("{} Closing server.", "[Mech Server]".bright_cyan());
    std::process::exit(0);

    None
  // The testing framework
  } else if let Some(matches) = matches.subcommand_matches("test") {
      println!("Testing...");
      let mech_paths = matches.values_of("mech_test_file_paths").map_or(vec![], |files| files.collect());
      let mut passed_all_tests = true;

      let mut compiler = Compiler::new();
      let code = read_mech_files(mech_paths).await?;
      let blocks = compile_code(code);

      let mut core = Core::new(1000,1000);
      core.register_blocks(blocks);
      core.step();

      let mut tests_count = 0;
      let mut tests_passed = 0;
      let mut tests_failed = 0;
      match core.get_table("mech/test".to_string()) {
        Some(test_results) => {
          let test_results = test_results.borrow();
          for i in 0..test_results.rows as usize {
            for j in 0..test_results.columns as usize {
              tests_count += 1;
              if test_results.data[j][i] == Value::Bool(false) {
                passed_all_tests = false;
                tests_failed += 1;
              } else {
                tests_passed += 1;
              }
            }
          }
        },
        _ => (),
      }


      if passed_all_tests {
        println!("Test result: {} | total {} | passed {} | failed {} | ", "ok".green(), tests_count, tests_passed, tests_failed);
        std::process::exit(0);
      } else {
        println!("Test result: {} | total {} | passed {} | failed {} | ", "failed".red(), tests_count, tests_passed, tests_failed);
        std::process::exit(1);
      }
      None
  } else if let Some(matches) = matches.subcommand_matches("run") {
    let mech_paths = matches.values_of("mech_run_file_paths").map_or(vec![], |files| files.collect());
    let repl = matches.is_present("repl_mode");    
    let code = read_mech_files(mech_paths).await?;
    let blocks = compile_code(code);

    // Spin up a mech core and add the new blocks
    let mut core = Core::new(1000,1000);
    core.register_blocks(blocks);
    let output_id: u64 = Hasher::hash_str("mech/output");  

    if !repl {
      println!("{}", "[Running]".bright_green());
      core.step();
      match core.store.get_table(output_id) {
        Some(output_table) => {
          let output_table = output_table.borrow();
          for i in 0..output_table.rows as usize {
            for j in 0..output_table.columns as usize {
              println!("{:?}", output_table.data[j][i]);
            }
          }
        },
        _ => (),
      }
      std::process::exit(0);
    }
    Some(core)
  // Build a .blx file from .mec and other .blx files
  } else if let Some(matches) = matches.subcommand_matches("build") {
    let mech_paths = matches.values_of("mech_build_file_paths").map_or(vec![], |files| files.collect());
    let code = read_mech_files(mech_paths).await?;
    let blocks = compile_code(code);

    let output_name = match matches.value_of("output_name") {
      Some(name) => format!("{}.blx",name),
      None => "output.blx".to_string(),
    };

    let file = OpenOptions::new().write(true).create(true).open(&output_name).unwrap();
    let mut writer = BufWriter::new(file);
    let mut miniblocks: Vec<MiniBlock> = Vec::new();
    for block in blocks {
      let mut miniblock = MiniBlock::new();
      miniblock.constraints = block.constraints.clone();
      miniblocks.push(miniblock);
    }
    let result = bincode::serialize(&miniblocks).unwrap();
    if let Err(e) = writer.write_all(&result) {
      panic!("{} Failed to write core! {:?}", "[Error]".bright_red(), e);
    }
    writer.flush().unwrap();

    println!("{} Wrote {}", "[Finished]".bright_green(), output_name);
    std::process::exit(0);
    None
  } else {
    None
  };

  println!("\n {}",  "╔═══════════════════════╗".bright_black());
  println!(" {}      {}      {}", "║".bright_black(), format!("MECH v{}",version).bright_yellow(), "║".bright_black());
  println!(" {}\n",  "╚═══════════════════════╝".bright_black());

    println!("Prepend commands with a colon. Enter :help to see a full list of commands. Enter :quit to quit.\n");
    let help_message = r#"
Available commands are: 

help    - displays this message
quit    - quits this REPL
core    - prints info about the current mech core
runtime - prints info about the runtime attached to the current core
pause   - pause core execution
resume  - resume core execution
clear   - reset the current core
"#;




  let runner = ProgramRunner::new("Mech REPL", 1000);
  let mech_client = runner.run();

  match core {
    Some(core) => {
      let mut miniblocks = Vec::new();
      for (_, block) in core.runtime.blocks.iter() {
        let mut miniblock = MiniBlock::new();
        miniblock.constraints = block.constraints.clone();
        miniblocks.push(miniblock);
      }
      mech_client.send(RunLoopMessage::Blocks(miniblocks));
    }
    None =>(),
  }

  let mut skip_receive = false;

  //ClientHandler::new("Mech REPL", None, None, None, cores);
  let formatted_name = format!("\n[{}]", mech_client.name).bright_cyan();
  let thread_receiver = mech_client.incoming.clone();
  
  // Break out receiver into its own thread
  let thread = thread::Builder::new().name("Mech Receiving Thread".to_string()).spawn(move || {
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
          print!("{}", ">: ".bright_yellow());
        },
        (Ok(ClientMessage::Table(table))) => {
          match table {
            Some(table) => {
              println!("{} ", formatted_name);
              print_table(&table);
              print!("{}", ">: ".bright_yellow());
            }
            None => println!("{} Table not found", formatted_name),
          }
        },
        (Ok(ClientMessage::Transaction(txn))) => {
          println!("{} Transaction: {:?}", formatted_name, txn);
        },
        (Ok(ClientMessage::Done)) => {
          // Do nothing
        },
        (Err(x)) => {
          println!("{} {}", "[Error]".bright_red(), x);
          break 'receive_loop;
        }
        q => {
          println!("else: {:?}", q);
        },
      };
      io::stdout().flush().unwrap();
    }
  });


  'REPL: loop {
     
    io::stdout().flush().unwrap();
    // Print a prompt
    print!("{}", ">: ".bright_yellow());
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
            break 'REPL;
          },
          ReplCommand::Table(id) => {
            mech_client.send(RunLoopMessage::Table(id));
          },
          ReplCommand::Clear => {
            mech_client.send(RunLoopMessage::Clear);
          },
          ReplCommand::PrintCore(core_id) => {
            mech_client.send(RunLoopMessage::PrintCore(core_id));
          },
          ReplCommand::PrintRuntime => {
            mech_client.send(RunLoopMessage::PrintRuntime);
          },
          ReplCommand::Pause => {mech_client.send(RunLoopMessage::Pause);},
          ReplCommand::Resume => {mech_client.send(RunLoopMessage::Resume);},
          ReplCommand::Empty => {
            println!("Empty");
          },
          ReplCommand::Error => {
            println!("Unknown command. Enter :help to see available commands.");
          },
          ReplCommand::Code(code) => {
            mech_client.send(RunLoopMessage::Code((0,MechCode::String(code))));
          },
          ReplCommand::EchoCode(code) => {
            mech_client.send(RunLoopMessage::EchoCode(code));
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

pub fn print_table(table: &Table) {
  // Get the length of each column
  let mut column_widths = vec![0; table.columns as usize];
  for column in 0..table.columns as usize {
    for row in 0..table.rows as usize {
      let value = match &table.data[column][row] {
        Value::Number(q) => format!("{}", q.to_float()),
        q => format!("{:?}", q),
      };
      if value.len() > column_widths[column] {
        column_widths[column] = value.len();
      }
    }
  }
  // Print the top border
  print!("┌");
  for i in 0 .. table.columns as usize - 1 {
    print_repeated_char("─", column_widths[i]);
    print!("┬");
  }
  print_repeated_char("─", column_widths[column_widths.len() - 1]);
  print!("┐\n");
  // Print each row
  for row in 0..table.rows as usize {
    print!("│");
    for column in 0..table.columns as usize {
      let content_string = match &table.data[column][row] {
        Value::Number(q) => format!("{}", q.to_float()),
        q => format!("{:?}", q),
      };
      print!("{}", content_string);
      // print padding
      print_repeated_char(" ", column_widths[column] - content_string.len());
      print!("│");
    }
    print!("\n");
  }  
  // Print the bottom border
  print!("└");
  for i in 0 .. table.columns as usize - 1 {
    print_repeated_char("─", column_widths[i]);
    print!("┴");
  }
  print_repeated_char("─", column_widths[column_widths.len() - 1]);
  print!("┘\n");
}

fn print_repeated_char(to_print: &str, n: usize) {
  for _ in 0..n {
    print!("{}", to_print);
  }
}

pub fn mech_code(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  // Try parsing mech code
  let mut parser = Parser::new();
  match parser.parse_fragment(input) {
    Ok(_) => Ok((input, ReplCommand::Code(input.to_string()))),
    Err(_) => {
      // Try parsing it as an anonymous statement
      let command = format!("#ans = {}", input.trim());
      let mut parser = Parser::new();
      match parser.parse_fragment(&command) { 
        Ok(_) => Ok((input, ReplCommand::EchoCode(command.to_string()))),
        Err(_) => Ok((input, ReplCommand::Error)),
      }
    }
  }
}

pub fn clear(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("clear")(input)?;
  Ok((input, ReplCommand::Clear))
}

pub fn runtime(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("runtime")(input)?;
  Ok((input, ReplCommand::PrintRuntime))
}

pub fn core(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("core")(input)?;
  let (input, _) = space0(input)?;
  let (input, core_id) = opt(digit1)(input)?;
  let core_id = match core_id {
    Some(core_id) => Some(core_id.parse::<u64>().unwrap()),
    None => None,
  };
  Ok((input, ReplCommand::PrintCore(core_id)))
}

pub fn quit(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = alt((tag("quit"),tag("exit")))(input)?;
  Ok((input, ReplCommand::Quit))
}

pub fn resume(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("resume")(input)?;
  Ok((input, ReplCommand::Resume))
}

pub fn pause(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("pause")(input)?;
  Ok((input, ReplCommand::Pause))
}

pub fn help(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag("help")(input)?;
  Ok((input, ReplCommand::Help))
}

pub fn command(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, _) = tag(":")(input)?;
  let (input, command) = alt((quit, help, pause, resume, core, runtime, clear))(input)?;
  Ok((input, command))
}

pub fn parse_repl_command(input: &str) -> IResult<&str, ReplCommand, VerboseError<&str>> {
  let (input, command) = alt((command, mech_code))(input)?;
  Ok((input, command))
}

fn print_errors(program: &Program) {
  if program.errors.len() > 0 {
    let plural = if program.errors.len() == 1 {
      ""
    } else {
      "s"
    };
    let error_notice = format!("Found {} Error{}:", &program.errors.len(), plural);
    println!("\n{}\n", error_notice.bright_red());
    for error in &program.errors {
      let block = &program.mech.runtime.blocks.get(&(error.block as usize)).unwrap();
      println!("{} {} {} {}\n ", "--".bright_yellow(), "Block".yellow(), block.name, "---------------------------------------".bright_yellow());
      match error.error_id {
        ErrorType::DuplicateAlias(alias_id) => {
          let alias = &program.mech.store.names.get(&alias_id).unwrap();
          println!(" Local table {:?} defined more than once.", alias);
        },
        _ => (),
      }
      println!("");
      for (ix,(text, constraints)) in block.constraints.iter().enumerate() {
        if constraints.contains(&error.constraint) {
          println!(" {} {}", ">".bright_red(), text);
        } else {
          println!("   {}", text.bright_black());
        }
      }
      println!("\n{}", "------------------------------------------------------\n".bright_yellow());
    }
  }
}