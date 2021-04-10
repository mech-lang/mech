// # Mech

/*
 Mech Server is a wrapper around the mech runtime. It provides interfaces for 
 controlling the runtime, sending it transactions, and responding to changes.
*/

// ## Prelude

extern crate core;
use std::io;

extern crate seahash;

extern crate clap;
use clap::{Arg, App, ArgMatches, SubCommand};

extern crate colored;
use colored::*;

extern crate mech;
use mech::{
  Core, 
  Change,
  Transaction,
  Index,
  ValueMethods,
  MiniBlock, 
  Block, 
  Transformation, 
  Compiler, 
  Table, 
  Value, 
  ParserNode, 
  hash_string, 
  Program, 
  ErrorType, 
  ProgramRunner, 
  RunLoop, 
  RunLoopMessage, 
  ClientMessage, 
  Parser,
  MechCode,
  WebsocketMessage,
  compile_code,
  read_mech_files,
  ReplCommand,
  parse_repl_command,
};
use mech::QuantityMath;

use std::thread::{self, JoinHandle};


extern crate reqwest;
use std::collections::HashMap;

extern crate bincode;
use std::io::{Write, BufReader, BufWriter, stdout};
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
extern crate actix;
extern crate actix_web_actors;
use actix::prelude::*;
use actix_web::{get, web, App as ActixApp, HttpServer, HttpResponse, Responder, Error, HttpRequest};
use actix_session::{CookieSession, Session};
use actix::{Actor, StreamHandler};
use actix_web_actors::ws;
//extern crate ws;
//use ws::{listen, Handler as WsHandler, Sender as WsSender, Result as WsResult, Message as WsMessage, Handshake, CloseCode, Error as WsError};
use std::time::Duration;

use std::rc::Rc;
use std::cell::Cell;

#[macro_use]
extern crate crossbeam_channel;
use crossbeam_channel::{Sender, Receiver};

extern crate tui;
use tui::backend::CrosstermBackend;
use tui::Terminal;
use tui::widgets::{Widget, Block as TuiBlock, Borders};
use tui::layout::{Layout, Constraint, Direction};

use crossterm::{
  ExecutableCommand, QueueableCommand,
  terminal, cursor, style::Print,
};

const MECH_TEST: u64 = 0xbfa7e6bd3c6fc7;
const NAME: u64 = 0x4d60aa46a343df;
const RESULT: u64 = 0xf039b7315b95ce;

// ## Mech Entry
#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

  #[cfg(windows)]
  control::set_virtual_terminal(true).unwrap();
  let version = "0.0.6";
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
    let full_address = format!("{}:{}",address,port);
    let mech_paths = matches.values_of("mech_serve_file_paths").map_or(vec![], |files| files.collect());
    let persistence_path = matches.value_of("persistence").unwrap_or("");

    // Spin up a mech core and compiler
    let mut core = Core::new(1000, 1000);
    core.load_standard_library();
    let code = read_mech_files(mech_paths).await?;
    let blocks = compile_code(code.clone());

    let mut miniblocks: Vec<MiniBlock> = Vec::new();
    for block in blocks {
      let mut miniblock = MiniBlock::new();
      miniblock.transformations = block.transformations.clone();
      miniblock.plan = block.plan.clone();
      for (k,v) in block.store.strings.iter() {
        miniblock.strings.push((k.clone(), v.clone()));
      }
      for (k,v) in block.store.number_literals.iter() {
        miniblock.number_literals.push((k.clone(), v.clone()));
      }
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

    async fn tables(
      session: Session, 
      req: web::HttpRequest, 
      info: web::Path<(String)>, 
      data: web::Data<(Sender<RunLoopMessage>,Receiver<ClientMessage>,Vec<u8>,Addr<ServerMonitor>)>
    ) -> impl Responder {
      println!("Serving");
      use core::hash::Hasher;
      //println!("Connection Info {:?}", req.connection_info());
      //println!("Head {:?}", req.head());
      let mut id: u64 = seahash::hash(format!("{:?}", req.head()).as_bytes());
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

    async fn serve_blocks(data: web::Data<(Sender<RunLoopMessage>,Receiver<ClientMessage>,Vec<u8>,Addr<ServerMonitor>)>) -> impl Responder {
      let (sender, receiver, miniblocks, _) = data.get_ref();
      format!("{{\"blocks\": {:?} }}", miniblocks)
    }

    #[derive(Message)]
    #[rtype(result = "()")]
    struct RegisterWSClient {
        addr: Addr<MyWebSocket>,
    }

    #[derive(Message, Debug)]
    #[rtype(result = "()")]
    struct ServerEvent {
        event: Vec<u8>,
    }

    async fn ws_index(
      r: HttpRequest, 
      stream: web::Payload, 
      data: web::Data<(Sender<RunLoopMessage>,Receiver<ClientMessage>,Vec<u8>,Addr<ServerMonitor>)>,
    ) -> Result<HttpResponse, Error> {
      let (outgoing, _, _, monitor) = data.get_ref();
      let (addr, res) = ws::start_with_addr(MyWebSocket {mech_outgoing: outgoing.clone()}, &r, stream).unwrap();
      monitor.do_send(RegisterWSClient { addr: addr });
      Ok(res)
    }


    struct MyWebSocket {
      mech_outgoing: Sender<RunLoopMessage>,
    }

    impl Actor for MyWebSocket {
      type Context = ws::WebsocketContext<Self>;
    }

    impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWebSocket {
      fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
      ) {
        // process websocket messages
        println!("WS: {:?}", msg);
        match msg {
          Ok(ws::Message::Ping(msg)) => {
            ctx.pong(&msg);
          }
          Ok(ws::Message::Pong(_)) => {
            ctx.text("Message!");
          }
          Ok(ws::Message::Text(text)) => ctx.text(text),
          Ok(ws::Message::Binary(bin)) => {
            let message: WebsocketMessage = bincode::deserialize(&bin).unwrap();
            match message {
              WebsocketMessage::Listening(register) => {
                self.mech_outgoing.send(RunLoopMessage::Listening(register));
              },
              _ => (),
            }
          },
          Ok(ws::Message::Close(_)) => {
              ctx.stop();
          }
          _ => ctx.stop(),
        }
      }
    }

    impl Handler<ServerEvent> for MyWebSocket {
      type Result = ();

      fn handle(&mut self, msg: ServerEvent, ctx: &mut Self::Context) {
        println!("Handling a server event: {:?}", msg);
        ctx.binary(msg.event);
      }
    }

    struct ServerMonitor {
      listeners: Vec<Addr<MyWebSocket>>,
      incoming: Receiver<Vec<u8>>,
    }

    impl Actor for ServerMonitor {
      type Context = Context<Self>;

      fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_millis(10), |act, _| {
          loop {
            match &act.incoming.try_recv() {
              Ok(message) => {
                println!("GOTA A MESSAGE {:?}", message);
                for listener in &act.listeners {
                  listener.do_send(ServerEvent{ event: message.to_vec() });
                }
              }
              Err(_) => break,
            }
          }
        });
      }
    }

    impl Handler<RegisterWSClient> for ServerMonitor {
        type Result = ();

        fn handle(&mut self, msg: RegisterWSClient, _: &mut Context<Self>) {
          println!("Adding a new connection to the server monitor");
          self.listeners.push(msg.addr);
        }
    }

    let (ws_outgoing, ws_incoming) = crossbeam_channel::unbounded();
    let thread_receiver = mech_client.incoming.clone();
    // Start a receive loop. Generally this will just pass messages on to the websocket client:
    let thread = thread::Builder::new().name("Websocket Receiving Thread".to_string()).spawn(move || {
      // Get all responses from the thread
      'receive_loop: loop {
        match thread_receiver.recv() {
          (Ok(ClientMessage::Pause)) => {
            //println!("{} Paused", formatted_name);
          },
          (Ok(ClientMessage::Resume)) => {
            //println!("{} Resumed", formatted_name);
          },
          (Ok(ClientMessage::Clear)) => {
            //println!("{} Cleared", formatted_name);
          },
          (Ok(ClientMessage::NewBlocks(count))) => {
            //println!("Compiled {} blocks.", count);
          },
          (Ok(ClientMessage::String(message))) => {
            //println!("{} {}", formatted_name, message);
            //print!("{}", ">: ".truecolor(246,192,78));
          },
          /*(Ok(ClientMessage::Table(table))) => {
            // Send the table received from the client over the websocket
            match table {
              Some(table) => {
                let ntable = NetworkTable::new(&table);
                let msg: Vec<u8> = bincode::serialize(&WebsocketMessage::Table(ntable)).unwrap();
                ws_outgoing.send(msg);
                //ctx.binary(msg);
              }
              None => (),
            }
          },*/
          (Ok(ClientMessage::Transaction(txn))) => {
            //println!("{} Transaction: {:?}", formatted_name, txn);
          },
          (Ok(ClientMessage::Done)) => {
            // Do nothing
          },
          (Err(x)) => {
            //println!("{} {}", "[Error]".bright_red(), x);
            break 'receive_loop;
          }
          q => {
            //println!("else: {:?}", q);
          },
        };
      }
    }).unwrap();

    println!("{} Awaiting connection at {}", "[Mech Server]".bright_cyan(), full_address);
    let srvmon = ServerMonitor { listeners: vec![], incoming: ws_incoming }.start();
    let data = web::Data::new((
      mech_client.outgoing.clone(), 
      mech_client.incoming.clone(), 
      serialized_miniblocks.clone(),
      srvmon.clone(),
    ));
    HttpServer::new(move || {
        ActixApp::new()
          .app_data(data.clone())
          .wrap(CookieSession::signed(&[0; 32]).secure(false))
          .service(web::resource("/tables/{query}").route(web::get().to(tables)))
          .service(web::resource("/blocks").route(web::get().to(serve_blocks)))
          // websocket route
          .service(web::resource("/ws/").route(web::get().to(ws_index)))
          .service(actix_files::Files::new("/", "./notebook/").index_file("index.html"))
          // static files
          //.service(fs::Files::new("/", "static/").index_file("index.html"))
    })
    .bind(full_address)?
    .run()
    .await?;
    println!("{} Closing server.", "[Mech Server]".bright_cyan());
    std::process::exit(0);

    None
  // ------------------------------------------------
  // TEST
  // ------------------------------------------------
  } else if let Some(matches) = matches.subcommand_matches("test") {
    println!("{}", "[Testing]".bright_green());
    let mech_paths = matches.values_of("mech_test_file_paths").map_or(vec![], |files| files.collect());
    let mut passed_all_tests = true;

    let mut compiler = Compiler::new();
    let code = read_mech_files(mech_paths).await?;
    let blocks = compile_code(code);

    let mut core = Core::new(1000, 1000);
    core.load_standard_library();
    let txn = Transaction{changes:
      vec![
        Change::NewTable{table_id: MECH_TEST, rows: 0, columns: 2},
        Change::SetColumnAlias{table_id: MECH_TEST, column_ix: 1, column_alias: NAME},
        Change::SetColumnAlias{table_id: MECH_TEST, column_ix: 2, column_alias: RESULT},
    ]};
    core.process_transaction(&txn);
    core.register_blocks(blocks);
    
    let mut tests_count = 0;
    let mut tests_passed = 0;
    let mut tests_failed = 0;
    
    match core.get_table(hash_string("mech/test")) {
      Some(test_results) => {

        println!("Running {} tests...\n", test_results.rows);

        for i in 1..=test_results.rows as usize {
          tests_count += 1;
          
          let test_name = match test_results.get(&TableIndex::Index(i),&TableIndex::Index(1)).unwrap().as_string() {
            Some(string_hash) => {
              test_results.get_string(&string_hash).unwrap().clone()
            }
            _ => "".to_string()
          };

          print!("\t{}\t\t", test_name);

          match test_results.get(&TableIndex::Index(i),&TableIndex::Index(2)).unwrap().as_bool() {
            Some(false) => {
              passed_all_tests = false;
              tests_failed += 1;
              println!("{}", "failed".red());

            },
            Some(true) => {
              tests_passed += 1;
              println!("{}", "ok".green());
            }
            _ => (),
          } 
        }
      },
      _ => (),
    }


    if passed_all_tests {
      println!("\nTest result: {} | total {} | passed {} | failed {} | \n", "ok".green(), tests_count, tests_passed, tests_failed);
      std::process::exit(0);
    } else {
      println!("\nTest result: {} | total {} | passed {} | failed {} | \n", "failed".red(), tests_count, tests_passed, tests_failed);
      std::process::exit(1);
    }
    None
  // ------------------------------------------------
  // RUN
  // ------------------------------------------------
  } else if let Some(matches) = matches.subcommand_matches("run") {
    let mech_paths = matches.values_of("mech_run_file_paths").map_or(vec![], |files| files.collect());
    let repl = matches.is_present("repl_mode");    
    let code = read_mech_files(mech_paths).await?;
    let blocks = compile_code(code);

    println!("{}", "[Running]".bright_green());
    let runner = ProgramRunner::new("Mech REPL", 1000);
    let mech_client = runner.run();
  
    let mut miniblocks = Vec::new();
    for block in &blocks {
      let mut miniblock = MiniBlock::new();
      miniblock.transformations = block.transformations.clone();
      miniblock.plan = block.plan.clone();
      for (k,v) in block.store.strings.iter() {
        miniblock.strings.push((k.clone(), v.clone()));
      }
      for (k,v) in block.store.number_literals.iter() {
        miniblock.number_literals.push((k.clone(), v.clone()));
      }
      miniblocks.push(miniblock);
    }
    mech_client.send(RunLoopMessage::Code((0, MechCode::MiniBlocks(miniblocks))));

    let mut skip_receive = false;
  
    //ClientHandler::new("Mech REPL", None, None, None, cores);
    let formatted_name = format!("\n[{}]", mech_client.name).bright_cyan();
    let thread_receiver = mech_client.incoming.clone();
    
    // Get all responses from the thread
    'receive_loop: loop {
      match thread_receiver.recv() {
        (Ok(ClientMessage::String(message))) => {
          println!("{} {}", formatted_name, message);
        },
        (Ok(ClientMessage::Table(table))) => {
          if !repl {
            match table {
              Some(table) => {
                println!("{:?}", table);
              }
              None => (), //println!("{} Table not found", formatted_name),
            }
            std::process::exit(0);
          } else {
            break 'receive_loop;
          }
        },
        (Ok(ClientMessage::Transaction(txn))) => {
          println!("{} Transaction: {:?}", formatted_name, txn);
        },
        (Ok(ClientMessage::Done)) => {
          // Do nothing
        },
        Ok(ClientMessage::StepDone) => {
          let output_id: u64 = hash_string("mech/output"); 
          mech_client.send(RunLoopMessage::GetTable(output_id));
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
    }
    Some(mech_client)
  // ------------------------------------------------
  // BUILD a .blx file from .mec and other .blx files
  // ------------------------------------------------
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
      miniblock.transformations = block.transformations.clone();
      miniblock.plan = block.plan.clone();
      for (k,v) in block.store.strings.iter() {
        miniblock.strings.push((k.clone(), v.clone()));
      }
      for (k,v) in block.store.number_literals.iter() {
        miniblock.number_literals.push((k.clone(), v.clone()));
      }
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
  } else if let Some(matches) = matches.subcommand_matches("clean") {
    std::fs::remove_dir_all("machines");
    std::process::exit(0);
    None
  } else {
    None
  };

let text_logo = r#"
  ┌───────┐     ┌────┐ ┌──────┐ ┌─┐   ┌─┐
  │ ┌─┐ ┌─┼─┐   └────┘ │ ┌────┘ │ │   │ │
  │ │ │ │ │ │ ┌──────┐ │ │      │ └─┐ │ │
  │ │ │ │ │ │ │ ┌────┘ │ │      │ ┌─┘ │ │
  │ │ └─┘ │ │ │ └────┐ │ └────┐ │ │   │ │
  └─┘     └─┘ └──────┘ └──────┘ └─┘   └─┘"#.truecolor(246,192,78);
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
      runner.run()
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
        (Ok(ClientMessage::Table(table))) => {
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
          //println!("else: {:?}", q);
        },
      };
      io::stdout().flush().unwrap();
    }
  });


  'REPL: loop {
     
    io::stdout().flush().unwrap();
    // Print a prompt
    print!("{}", ">: ".truecolor(246,192,78));
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
            //mech_client.send(RunLoopMessage::Table(id));
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

fn print_errors(program: &Program) {
  /*
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
      println!("{} {} {} {}\n ", "--".truecolor(246,192,78), "Block".yellow(), block.name, "---------------------------------------".truecolor(246,192,78));
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
      println!("\n{}", "------------------------------------------------------\n".truecolor(246,192,78));
    }
  }*/
}