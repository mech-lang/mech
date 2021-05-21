use mech_core::{Core, humanize, Register, Transaction, Change, Error, ErrorType};
use mech_core::{Block, BlockState};
use mech_core::{Table, TableId, TableIndex};
use mech_core::hash_string;
use mech_syntax::compiler::Compiler;
use mech_utilities::{RunLoopMessage, MechCode, Machine, MachineRegistrar, MachineDeclaration};

use std::thread::{self, JoinHandle};
use std::sync::Arc;
use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;
use colored::*;

use super::program::Program;
use super::persister::Persister;

use std::net::{SocketAddr, UdpSocket};
use std::io;

// ## Run Loop

// Client messages are sent to the client from the run loop

/*pub enum MechChannel {
  Crossbeam(crossbeam_channel::Sender<ClientMessage>),
  UdpSocket(UdpSocket),
}

impl MechChannel {

  pub fn send(&mut self, message: ClientMessage) { 
    match &self {
      MechChannel::Crossbeam(sender) => {
        sender.send(message);
      }
      MechChannel::UdpSocket(socket) => {
        let msg: Vec<u8> = bincode::serialize(&message).unwrap();
        socket.send(&msg);
      }
    }
  }
}*/

#[derive(Debug, Clone)]
pub enum ClientMessage {
  Stop,
  Pause,
  Resume,
  Clear,
  Exit(i32),
  Time(usize),
  NewBlocks(usize),
  Table(Option<Table>),
  Transaction(Transaction),
  String(String),
  //Block(Block),
  StepDone,
  Done,
  Ready,
}

pub struct RunLoop {
  pub name: String,
  thread: JoinHandle<()>,
  pub outgoing: Sender<RunLoopMessage>,
  pub incoming: Receiver<ClientMessage>,
}

impl RunLoop {

  pub fn wait(self) {
    self.thread.join().unwrap();
  }

  pub fn close(&self) {
    match self.outgoing.send(RunLoopMessage::Stop) {
      Ok(..) => (),
      Err(..) => (),
    }
  }

  pub fn send(&self, msg: RunLoopMessage) -> Result<(),&str> {
    match self.outgoing.send(msg) {
      Ok(_) => Ok(()),
      Err(_) => Err("Failed to send message"),
    }
  }

  pub fn receive(&self) -> Result<ClientMessage,&str> {
    match self.incoming.recv() {
      Ok(message) => Ok(message),
      Err(_) => Err("Failed to send message"),
    }
  }

  pub fn is_empty(&self) -> bool {
    self.incoming.is_empty()
  }

  pub fn channel(&self) -> Sender<RunLoopMessage> {
    self.outgoing.clone()
  }

}

// ## Program Runner

pub struct ProgramRunner {
  pub name: String,
  pub socket: io::Result<UdpSocket>,
  //pub persistence_channel: Option<Sender<PersisterMessage>>,
}

impl ProgramRunner {

  pub fn new(name:&str, capacity: usize) -> ProgramRunner {
    // Start a new program
    //let mut program = Program::new(name, capacity);

    // Start a persister
    /*
    let persist_name = format!("{}.mdb", name);
    let mut persister = Persister::new(&persist_name);
    persister.load(&persist_name);
    let changes = persister.get_changes();

    // Load database
    ////println!("{} Applying {} stored changes...", BrightCyan.paint(format!("[{}]", name)), changes.len());    
    for change in changes {
      program.mech.process_transaction(&Transaction::from_change(change));
    }*/
    
    ProgramRunner {
      name: name.to_owned(),
      socket: UdpSocket::bind("127.0.0.1:0"),
      //program,
      // TODO Use the persistence file specified by the user
      //persistence_channel: Some(persister.get_channel()),
      //persistence_channel: None,
    }
  }

  /*pub fn load_program(&mut self, input: String) -> Result<(),Box<std::error::Error>> {
    self.program.compile_program(input);
    Ok(())
  }

  pub fn load_core(&mut self, mut core: Core) {
    core.id = (self.program.cores.len() + 1) as u64;
    for input_register in &core.input {
      let input = self.program.input_map.entry(input_register.clone()).or_insert(HashSet::new());
      input.insert(core.id);
    }

    let table = core.get_table("#data".to_string()).unwrap();
    self.program.mech.remote_tables.push(table.clone());

    self.program.cores.insert(core.id, core);
  }*/

  pub fn add_persist_channel(&mut self, persister:&mut Persister) {
    //self.persistence_channel = Some(persister.get_channel());
  }

  pub fn run(self) -> RunLoop {
    //let name = self.name;
    //let outgoing = self.program.outgoing.clone();
    let (outgoing, program_incoming) = crossbeam_channel::unbounded();
    let runloop_outgoing = outgoing.clone();
    let (client_outgoing, incoming) = crossbeam_channel::unbounded();
    //let mut program = self.program;
    //let persistence_channel = self.persistence_channel;

    let thread = thread::Builder::new().name(self.name.to_owned()).spawn(move || {

      let mut program = Program::new("new program", 100, 1000, outgoing.clone(), program_incoming);

      //program.download_dependencies(Some(client_outgoing.clone()));

      // Step cores
      program.mech.step();
      for core in program.cores.values_mut() {
        core.step();
      }

      // Send the ready to the client to indicate that the program is initialized
      client_outgoing.send(ClientMessage::Ready);
      let mut paused = false;
      'runloop: loop {
        match (program.incoming.recv(), paused) {
          (Ok(RunLoopMessage::Transaction(txn)), false) => {
            use std::time::Instant;
            let start_ns = time::precise_time_ns();
            program.mech.process_transaction(&txn);
            let end_ns = time::precise_time_ns();
            let time = (end_ns - start_ns) as f64;   
            program.trigger_machines();  
            //println!("{:?}", program.mech);
            println!("Txn took {:0.4?} ms", time / 1_000_000.0);
            //println!("{}", program.mech.get_table("ball".to_string()).unwrap().borrow().rows);
            /*let mut changes: Vec<Change> = Vec::new();
            for i in pre_changes..program.mech.store.len() {
              let change = &program.mech.store.changes[i-1];
              match change {
                Change::Set{table, ..} => {
                  match program.listeners.get(&TableId::Global(*table)) {
                    Some(_) => changes.push(change.clone()),
                    _ => (),
                  }
                }
                _ => ()
              } 
            }
            if !changes.is_empty() {
              let txn = Transaction::from_changeset(changes);
              client_outgoing.send(ClientMessage::Transaction(txn));
            }*/
            client_outgoing.send(ClientMessage::StepDone);
          },
          (Ok(RunLoopMessage::Listening(ref register)), _) => {
            println!("Someone is listening for: {:?}", register);
            /*
            match program.mech.output.get(register) {
              Some(_) => {
                // We produce a table for which they're listening, so let's mark that
                // so we can send updates
                program.listeners.insert(register.clone()); 
                // Send over the table we have now
                let table_ref = program.mech.get_table_by_id(&register.table);
                client_outgoing.send(ClientMessage::Table(Some(table_ref.unwrap().borrow().clone())));
              }, 
              _ => (),
            }*/
            
          },
          (Ok(RunLoopMessage::Stop), _) => { 
            client_outgoing.send(ClientMessage::Stop);
            break 'runloop;
          },
          /*(Ok(RunLoopMessage::GetTable(table_id)), _) => { 
            let table_msg = ClientMessage::Table(program.mech.get_table(table_id));
            client_outgoing.send(table_msg);
          },*/
          (Ok(RunLoopMessage::Pause), false) => { 
            paused = true;
            client_outgoing.send(ClientMessage::Pause);
          },
          (Ok(RunLoopMessage::Resume), true) => {
            paused = false;
            //program.mech.resume();
            client_outgoing.send(ClientMessage::Resume);
          },
          (Ok(RunLoopMessage::StepBack), _) => {
            if !paused {
              paused = true;
            }
            //program.mech.step_back_one();
            //client_outgoing.send(ClientMessage::Time(program.mech.offset));
          }
          (Ok(RunLoopMessage::StepForward), true) => {
            //program.mech.step_forward_one();
            //client_outgoing.send(ClientMessage::Time(program.mech.offset));
          } 
          (Ok(RunLoopMessage::RemoteCore(remote_core_address)), _) => {
            match self.socket {
              Ok(ref socket) => {
                if !program.remote_cores.contains(&remote_core_address) {
                  program.remote_cores.insert(remote_core_address.clone());
                  let buf: [u8;6] = [1,2,3,4,5,6];
                  let len = socket.send_to(&buf, remote_core_address).unwrap();
                  println!("Sent {:?}", len);
                }
              }
              Err(_) => (),
            }
          } 
          (Ok(RunLoopMessage::String((string,color))), _) => {
            let r: u8 = (color >> 16) as u8;
            let g: u8 = (color >> 8) as u8;
            let b: u8 = color as u8;
            let colored_string = format!("{}", string.truecolor(r,g,b));
            client_outgoing.send(ClientMessage::String(colored_string));
          } 
          (Ok(RunLoopMessage::Exit(exit_code)), _) => {
            client_outgoing.send(ClientMessage::Exit(exit_code));
          } 
          (Ok(RunLoopMessage::Code(code_tuple)), _) => {
            let block_count = program.mech.runtime.blocks.len();
            match code_tuple {
              (0, MechCode::String(code)) => {
                let mut compiler = Compiler::new(); 
                compiler.compile_string(code);
                program.mech.register_blocks(compiler.blocks);
                program.trigger_machines();
                program.download_dependencies(Some(client_outgoing.clone()));

                client_outgoing.send(ClientMessage::StepDone);
              },
              (0, MechCode::MiniBlocks(miniblocks)) => {
                let mut blocks: Vec<Block> = Vec::new();
                for miniblock in miniblocks {
                  let mut block = Block::new(100);
                  for tfms in miniblock.transformations {
                    block.register_transformations(tfms);
                  }
                  for error in miniblock.errors {
                    block.errors.insert(error.clone());
                    program.errors.insert(error);
                  }
                  block.plan = miniblock.plan.clone();
                  let store = unsafe{&mut *Arc::get_mut_unchecked(&mut block.store)};
                  for (key, value) in miniblock.strings {
                    store.strings.insert(key, value.to_string());
                  }
                  for (key, value) in miniblock.number_literals {
                    store.number_literals.insert(key, value.clone());
                  }
                  block.id = miniblock.id;
                  blocks.push(block);
                }
                program.mech.register_blocks(blocks);
                program.trigger_machines();
                program.download_dependencies(Some(client_outgoing.clone()));
                for error in &program.mech.runtime.errors {
                  program.errors.insert(error.clone());
                }
                if program.errors.len() > 0 {
                  let error_string = format_errors(&program);
                  client_outgoing.send(ClientMessage::String(error_string));
                  client_outgoing.send(ClientMessage::Exit(1));
                }
                client_outgoing.send(ClientMessage::StepDone);
              }
              (ix, code) => {

              }
            }
          }
          (Ok(RunLoopMessage::EchoCode(code)), _) => {
            
            // Reset #ans
            program.mech.clear_table(hash_string("ans"));

            // Compile and run code
            let mut compiler = Compiler::new();
            compiler.compile_string(code);
            program.mech.register_blocks(compiler.blocks);
            program.download_dependencies(Some(client_outgoing.clone()));

            // Get the result
            let echo_table = program.mech.get_table(hash_string("ans"));
            program.listeners.insert(Register{table_id: TableId::Global(hash_string("ans")), row: TableIndex::All, column: TableIndex::All }); 

            // Send it
            //client_outgoing.send(ClientMessage::Table(echo_table));
            client_outgoing.send(ClientMessage::StepDone);
          } 
          (Ok(RunLoopMessage::Clear), _) => {
            program.clear();
            client_outgoing.send(ClientMessage::Clear);
          },
          (Ok(RunLoopMessage::PrintCore(core_id)), _) => {
            match core_id {
              None => client_outgoing.send(ClientMessage::String(format!("There are {:?} cores running.", program.cores.len() + 1))),
              Some(0) => client_outgoing.send(ClientMessage::String(format!("{:?}", program.mech))),
              Some(core_id) => client_outgoing.send(ClientMessage::String(format!("{:?}", program.cores.get(&core_id)))),
            };
          },
          (Ok(RunLoopMessage::PrintRuntime), _) => {
            //println!("{:?}", program.mech.runtime);
            client_outgoing.send(ClientMessage::String(format!("{:?}",program.mech.runtime)));
          },
          (Ok(RunLoopMessage::Blocks(miniblocks)), _) => {
            let mut blocks: Vec<Block> = Vec::new();
            for miniblock in miniblocks {
              let mut block = Block::new(100);
              for tfms in miniblock.transformations {
                block.register_transformations(tfms);
              }
              blocks.push(block);
            }
            program.mech.register_blocks(blocks);
            program.mech.step();
            client_outgoing.send(ClientMessage::StepDone);
          }
          (Err(_), _) => {
            break 'runloop
          },
          x => println!("qq{:?}", x),
        }
        //println!("{:?}", program.mech);
        client_outgoing.send(ClientMessage::Done);
      }
      /*if let Some(channel) = persistence_channel {
        channel.send(PersisterMessage::Stop);
      }*/
    }).unwrap();
    RunLoop { name: "Foo".to_string(), thread, outgoing: runloop_outgoing, incoming }
  }

  /*pub fn colored_name(&self) -> term_painter::Painted<String> {
    BrightCyan.paint(format!("[{}]", &self.name))
  }*/

}


fn format_errors(program: &Program) -> String {
  let mut formatted_errors = "".to_string();
  if program.errors.len() > 0 {
    let plural = if program.errors.len() == 1 {
      ""
    } else {
      "s"
    };
    let error_notice = format!("Found {} Error{}:\n", &program.errors.len(), plural);
    formatted_errors = format!("{}\n{}\n\n", formatted_errors, error_notice.bright_red());
    for error in &program.errors {
      let block = &program.mech.runtime.blocks.get(&error.block_id).unwrap();
      formatted_errors = format!("{}{} {} {} {}\n\n", formatted_errors, "--".truecolor(246,192,78), "Block".truecolor(246,192,78), block.name, "--------------------------------------------".truecolor(246,192,78));
      match &error.error_type {
        ErrorType::DuplicateAlias(alias_id) => {
          let alias = &program.mech.get_string(&alias_id).unwrap();
          formatted_errors = format!("{} Local table {:?} defined more than once.\n",formatted_errors, alias);
        },
        ErrorType::MissingFunction(function_id) => {
          let missing_function = &program.mech.get_string(&function_id).unwrap();
          formatted_errors = format!("{} Missing function: {}()\n",formatted_errors, missing_function);
        },
        ErrorType::UnsatisfiedTransformation(missing_ids) => {
          formatted_errors = format!("{} Missing:",formatted_errors);
          for id in missing_ids {
            let missing_string = &program.mech.get_string(&id).unwrap();
            formatted_errors = format!("{} {}",formatted_errors,missing_string);
          }
        },
        _ => formatted_errors = format!("{}{:?}\n", formatted_errors, error),
      }
      formatted_errors = format!("{}\n", formatted_errors);
      formatted_errors = format!("{} {} {}\n",formatted_errors, ">".bright_red(), error.step_text);
      formatted_errors = format!("{}\n{}",formatted_errors, "------------------------------------------------------\n\n".truecolor(246,192,78));
    }
  }
  formatted_errors
}