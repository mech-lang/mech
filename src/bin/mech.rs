// # Mech

/*
 Mech Server is a wrapper around the mech runtime. It provides interfaces for 
 controlling the runtime, sending it transactions, and responding to changes.
*/

// ## Prelude

extern crate core;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::io;
use std::io::prelude::*;
use std::fs::File;

extern crate clap;
use clap::{Arg, App, ArgMatches, SubCommand};

extern crate term_painter;
use term_painter::ToStyle;
use term_painter::Color::*;

extern crate mech;
use mech::{Core, Compiler, Table, Value, Hasher, ProgramRunner, RunLoop, RunLoopMessage, ClientMessage, Parser};
use mech::ClientHandler;
use mech::QuantityMath;

#[macro_use]
extern crate nom;
use nom::alpha1 as nom_alpha1;
use nom::digit1 as nom_digit1;
use nom::AtEof as eof;
use nom::types::CompleteStr;

extern crate mech_server;


#[derive(Debug, Clone)]
pub enum ReplCommand {
  Help,
  Quit,
  Pause,
  Resume,
  Stop,
  PrintCore,
  PrintRuntime,
  Clear,
  Table(u64),
  Code(String),
  Empty,
}

// ## Mech Entry

fn main() {
  let version = "0.0.3";
  let matches = App::new("Mech")
    .version(version)
    .author("Corey Montella")
    .about("The Mech REPL. Default values for options are in parentheses.")
    .arg(Arg::with_name("mech_file_paths")
      .help("The files and folders from which to load .mec files")
      .required(false)
      .multiple(true))
    .arg(Arg::with_name("serve")
      .short("s")
      .long("serve")
      .help("Starts a Mech HTTP and websocket server (false)"))
    .arg(Arg::with_name("port")
      .short("p")
      .long("port")
      .value_name("PORT")
      .help("Sets the port for the Mech server (3012)")
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
      .takes_value(true))
    .subcommand(SubCommand::with_name("test")
      .about("Execute all tests of a local package"))
    .subcommand(SubCommand::with_name("run")
      .about("Run a target folder or *.mec file")    
      .arg(Arg::with_name("mech_run_file_paths")
        .help("The files and folders to run.")
        .required(true)
        .multiple(true)))      
    .get_matches();

  let wport = matches.value_of("port").unwrap_or("3012");
  let hport = matches.value_of("http-port").unwrap_or("8081");
  let address = matches.value_of("address").unwrap_or("127.0.0.1");
  let serve = matches.is_present("serve");
  let http_address = format!("{}:{}",address,hport);
  let websocket_address = format!("{}:{}",address,wport);
  let mech_paths = matches.values_of("mech_file_paths").map_or(vec![], |files| files.collect());
  let persistence_path = matches.value_of("persistence").unwrap_or("");

  // The testing framework
  if let Some(matches) = matches.subcommand_matches("test") {
      println!("Testing...");
      let paths = std::fs::read_dir("./").unwrap();
      let mut passed_all_tests = true;
      let mut tests_count = 0;
      let mut tests_passed = 0;
      let mut tests_failed = 0;
      for path in paths {
        let path = path.unwrap().path();
        match (path.file_name(), path.extension())  {
          (Some(name), Some(extension)) => {
            match extension.to_str() {
              Some("mec") => {
                let mut f = File::open(name).unwrap();

                let mut buffer = String::new();

                f.read_to_string(&mut buffer);

                // Spin up a mech core and compiler
                let mut core = Core::new(1000,1000);
                let mut compiler = Compiler::new();
                compiler.compile_string(buffer);
                core.register_blocks(compiler.blocks);
                core.step();
                let test_results_id: u64 = Hasher::hash_str("mech/test");  

                match core.store.get_table(test_results_id) {
                  Some(test_results) => {
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
              }
              _ => (),
            }
          },
          _ => (),
        };
      }
      if passed_all_tests {
        println!("Test result: {} | total {} | passed {} | failed {} | ", Green.paint("ok"), tests_count, tests_passed, tests_failed);
        std::process::exit(0);
      } else {
        println!("Test result: {} | total {} | passed {} | failed {} | ", Red.paint("failed"), tests_count, tests_passed, tests_failed);
        std::process::exit(1);
      }
  } else if let Some(matches) = matches.subcommand_matches("run") {
    let mech_paths = matches.values_of("mech_run_file_paths").map_or(vec![], |files| files.collect());
    println!("Running {:?}", mech_paths);
    // TODO - Implement running a folder of .mec files
  
    for path_str in mech_paths {
      let path = Path::new(path_str);
      match (path.file_name(), path.extension())  {
        (Some(name), Some(extension)) => {
          match extension.to_str() {
            Some("mec") => {
              let mut f = File::open(name).unwrap();

              let mut buffer = String::new();

              f.read_to_string(&mut buffer);

              // Spin up a mech core and compiler
              let mut core = Core::new(1000,1000);
              let mut compiler = Compiler::new();
              compiler.compile_string(buffer);
              core.register_blocks(compiler.blocks);
              core.step();
              let output_id: u64 = Hasher::hash_str("mech/output");  

              match core.store.get_table(output_id) {
                Some(output_table) => {
                  for i in 0..output_table.rows as usize {
                    for j in 0..output_table.columns as usize {
                      println!("{:?}", output_table.data[j][i]);
                    }
                  }
                },
                _ => (),
              }
            }
            _ => (),
          }
        },
        _ => (),
      };
    }


    std::process::exit(0);
  }

  println!("\n {}",  BrightBlack.paint("╔═══════════════════════╗"));
  println!(" {}      {}      {}", BrightBlack.paint("║"), BrightYellow.paint(format!("MECH v{}",version)), BrightBlack.paint("║"));
  println!(" {}\n",  BrightBlack.paint("╚═══════════════════════╝"));
  if serve {
    mech_server::http_server(http_address);
    mech_server::websocket_server(websocket_address, mech_paths, persistence_path);
  // If we're not serving, go into a REPL
  } else {
    let paths = if mech_paths.is_empty() {
      None
    } else {
      Some(&mech_paths)
    };
    let mech_client = ClientHandler::new("Mech REPL", None, paths, None);
    'REPL: loop {

      // Print a prompt
      print!("{}", Yellow.paint(">: "));
      let mut input = String::new();

      io::stdin().read_line(&mut input).unwrap();

      // Handle built in commands
      let parse = parse_repl_command(CompleteStr(input.trim()));
      match parse {
        Ok((CompleteStr(""), command)) => {
          match command {
            ReplCommand::Help => {
              println!("Available commands are: help, quit, core, runtime, pause, resume");
              continue;
            },
            ReplCommand::Quit => {
              break 'REPL;
            },
            ReplCommand::Table(id) => {
              mech_client.running.send(RunLoopMessage::Table(id));
            },
            ReplCommand::Clear => {
              mech_client.running.send(RunLoopMessage::Clear);
            },
            ReplCommand::PrintCore => {
              mech_client.running.send(RunLoopMessage::PrintCore);
            },
            ReplCommand::PrintRuntime => {
              mech_client.running.send(RunLoopMessage::PrintRuntime);
            },
            ReplCommand::Pause => {mech_client.running.send(RunLoopMessage::Pause);},
            ReplCommand::Resume => {mech_client.running.send(RunLoopMessage::Resume);},
            ReplCommand::Empty => {
              println!("Empty");
              continue;
            },
            _ => {
              continue;
            }
          }
        },
        err => {
          if input.trim() == "" {
            continue;
          }
          // Try parsing mech code
          let mut parser = Parser::new();
          parser.parse(input.trim());
          if parser.unparsed == "" {
            mech_client.running.send(RunLoopMessage::Code(input.trim().to_string()));
          // Try parsing it as an anonymous statement
          } else {
            let command = format!("#ans = {}", input.trim());
            let mut parser = Parser::new();
            parser.parse(&command);
            if parser.unparsed == "" { 
              mech_client.running.send(RunLoopMessage::Code(command));
            } else {
                println!("{} Unknown Command: {:?}", Red.paint("Error:"), input.trim());
              continue;
            }
          }
        }, 
      }

      // Get a response from the thread
      match mech_client.running.receive() {
        (Ok(ClientMessage::Table(table))) => {
          match table {
            Some(ref table_ref) => print_table(table_ref),
            None => (),
          }
        },
        (Ok(ClientMessage::Pause)) => {
          println!("{} Paused", BrightCyan.paint(format!("[{}]", mech_client.client_name)));
        },
        (Ok(ClientMessage::Resume)) => {
          println!("{} Resumed", BrightCyan.paint(format!("[{}]", mech_client.client_name)));
        },
        (Ok(ClientMessage::Clear)) => {
          println!("{} Cleared", BrightCyan.paint(format!("[{}]", mech_client.client_name)));
        },
        (Ok(ClientMessage::NewBlocks(count))) => {
          println!("Compiled {} blocks.", count);
        },
        _ => (),
      };

    }
  }
}

fn print_table(table: &Table) {
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

named!(word<CompleteStr, String>, do_parse!(
  bytes: nom_alpha1 >>
  (bytes.to_string())));

named!(slash<CompleteStr, String>, do_parse!(
  tag!("/") >> ("/".to_string())));

named!(dash<CompleteStr, String>, do_parse!(
  tag!("-") >> ("-".to_string())));

named!(identifier<CompleteStr, String>, do_parse!(
  identifier: map!(tuple!(count!(word,1), many0!(alt!(dash | slash | word))), |tuple| {
    let (mut word, mut rest) = tuple;
    word.append(&mut rest);
    let word = word.iter().fold(String::new(), |mut word, next| {word.push_str(next); word});
    word
  }) >> (identifier)));

named!(table<CompleteStr, ReplCommand>, do_parse!(
  tag!("#") >> identifier: map!(identifier, |identifier| { Hasher::hash_string(identifier) }) >>
  (ReplCommand::Table(identifier))));

named!(space<CompleteStr, ReplCommand>, do_parse!(
  many1!(tag!(" ")) >> (ReplCommand::Empty)));

named!(empty<CompleteStr, ReplCommand>, do_parse!(
  space >> (ReplCommand::Empty)));

named!(resume<CompleteStr, ReplCommand>, do_parse!(
  tag!("resume") >> (ReplCommand::Resume)));

named!(pause<CompleteStr, ReplCommand>, do_parse!(
  tag!("pause") >> (ReplCommand::Pause)));

named!(quit<CompleteStr, ReplCommand>, do_parse!(
  tag!("quit") >> (ReplCommand::Quit)));

named!(core<CompleteStr, ReplCommand>, do_parse!(
  tag!("core") >> (ReplCommand::PrintCore)));

named!(clear<CompleteStr, ReplCommand>, do_parse!(
  tag!("clear") >> (ReplCommand::Clear)));

named!(runtime<CompleteStr, ReplCommand>, do_parse!(
  tag!("runtime") >> (ReplCommand::PrintRuntime)));

named!(help<CompleteStr, ReplCommand>, do_parse!(
  tag!("help") >> (ReplCommand::Help)));

named!(parse_repl_command<CompleteStr, ReplCommand>, do_parse!(
  command: alt!(help | quit | pause | resume | table | core | clear | runtime | empty) >>
  (command)));