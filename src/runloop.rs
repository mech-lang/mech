use mech_core::*;
use mech_syntax::compiler::Compiler;
use mech_utilities::*;

use std::thread::{self, JoinHandle};
use std::sync::Arc;
use std::cell::RefCell;
use hashbrown::{HashSet, HashMap};
use hashbrown::hash_map::Entry;
use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;
use colored::*;

use super::program::Program;
use super::persister::Persister;

use std::net::{SocketAddr, UdpSocket};
extern crate websocket;
use websocket::OwnedMessage;

use std::io;
use std::time::Instant;
use std::sync::Mutex;

extern crate miniz_oxide;

use miniz_oxide::inflate::decompress_to_vec;
use miniz_oxide::deflate::compress_to_vec;

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
  Value(Value),
  Transaction(Transaction),
  String(String),
  Error(MechError),
  Timing(f64),
  //Block(Block),
  StepDone,
  Done,
  Ready,
}

pub struct RunLoop {
  pub name: String,
  pub socket_address: Option<String>,
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
  pub socket: Option<Arc<UdpSocket>>,
  pub registry: String,
  //pub persistence_channel: Option<Sender<PersisterMessage>>,
}

impl ProgramRunner {

  pub fn new(name:&str) -> ProgramRunner {
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
    
    let socket = match UdpSocket::bind("127.0.0.1:0") {
      Ok(socket) => Some(Arc::new(socket)),
      _ => None,
    };

    ProgramRunner {
      name: name.to_owned(),
      socket,
      registry: "https://gitlab.com/mech-lang/machines/mech/-/raw/v0.1-beta/src/registry.mec".to_string(),
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

  pub fn run(self) -> Result<RunLoop,MechError> {
    //let name = self.name;
    //let outgoing = self.program.outgoing.clone();
    let (outgoing, program_incoming) = crossbeam_channel::unbounded();
    let runloop_outgoing = outgoing.clone();
    let (client_outgoing, incoming) = crossbeam_channel::unbounded();
    //let mut program = self.program;
    //let persistence_channel = self.persistence_channel;

    let name = format!("{}", &self.name.clone());
    let socket_address = match self.socket {
      Some(ref socket) => Some(socket.local_addr().unwrap().to_string()),
      None => None,
    };

    // Start a channel receiving thread    
    let thread = thread::Builder::new().name(name.clone()).spawn(move || {
      
      let mut program = Program::new("new program", 100, 1000, outgoing.clone(), program_incoming, self.registry);

      let program_channel_udpsocket = program.outgoing.clone();
      let program_channel_udpsocket = program.outgoing.clone();

      match &self.socket {
        Some(ref socket) => {
          let socket_receiver = socket.clone();
          // Start a socket receiving thread
          let thread = thread::Builder::new().name("remote core listener".to_string()).spawn(move || {
            let mut compressed_message = [0; 16_383];
            loop {
              match socket_receiver.recv_from(&mut compressed_message) {
                Ok((amt, src)) => {
                  let serialized_message = decompress_to_vec(&compressed_message).expect("Failed to decompress!");
                  let message: Result<SocketMessage, bincode::Error> = bincode::deserialize(&serialized_message);
                  match message {
                    Ok(SocketMessage::RemoteCoreConnect(remote_core_address)) => {
                      program_channel_udpsocket.send(RunLoopMessage::RemoteCoreConnect(MechSocket::UdpSocket(remote_core_address)));
                    }
                    Ok(SocketMessage::RemoteCoreDisconnect(remote_core_address)) => {
                      program_channel_udpsocket.send(RunLoopMessage::RemoteCoreDisconnect(remote_core_address));
                    }
                    Ok(SocketMessage::Listening(register)) => {
                      program_channel_udpsocket.send(RunLoopMessage::Listening((hash_str(&src.to_string()), register)));
                    }
                    Ok(SocketMessage::Ping) => {
                      println!("Got a ping from: {:?}", src);
                      let message = bincode::serialize(&SocketMessage::Pong).unwrap();
                      let compressed_message = compress_to_vec(&message,6);
                      socket_receiver.send_to(&compressed_message, src);
                    }
                    Ok(SocketMessage::Pong) => {
                      println!("Got a pong from: {:?}", src);
                    }
                    Ok(SocketMessage::Transaction(txn)) => {
                      program_channel_udpsocket.send(RunLoopMessage::String((format!("Received Txn: {:?}", txn),None)));
                      program_channel_udpsocket.send(RunLoopMessage::Transaction(txn));
                    }
                    Ok(x) => println!("Unhandled Message {:?}", x),
                    Err(error) => println!("{:?}", error),
                  }
                }
                Err(error) => {

                }
              }
            }
          }).unwrap();
        }
        None => (),
      }

      match program.download_dependencies(Some(client_outgoing.clone())) {
        Ok(resolved_errors) => {
          let (_,_,nbo) = program.mech.resolve_errors(&resolved_errors);
          program.mech.schedule_blocks();
          for output in nbo {
            program.mech.step(&output);
          }
        }
        Err(err) => {
          client_outgoing.send(ClientMessage::Error(err.clone()));
        }
      }

      // Step cores
      /*program.mech.step();
      for core in program.cores.values_mut() {
        core.step();
      }*/

      // Send the ready to the client to indicate that the program is initialized
      client_outgoing.send(ClientMessage::Ready);
      let mut paused = false;
      let mut iteration: u64 = 0;
      'runloop: loop {
        match (program.incoming.recv(), paused) {
          (Ok(RunLoopMessage::Transaction(txn)), false) => {
            // Process the transaction and calculate how long it took. 
            let start_ns = time::precise_time_ns();
            match program.mech.process_transaction(&txn) {
              Ok((new_block_ids,changed_registers)) => {
                for trigger_register in &changed_registers {                  
                  // Handle machines first
                  let mut machine_triggers = vec![];
                  match &program.mech.schedule.trigger_to_output.get(trigger_register) {
                    Some(ref output) => {
                      for register in output.iter() {
                        machine_triggers.push(register.clone());
                      }
                    }
                    None => ()
                  }
                  for register in machine_triggers {
                    program.trigger_machine(&register);
                  }

                  // We have a triggered register, and we need to get all of the
                  // blocks that it potentially updated. We already have that list. If this register
                  // has been triggered for the first time, then we need to get the list of
                  // output blocks
                  match program.trigger_to_listener.entry(trigger_register.clone()) {
                    // Already triggered in the past
                    Entry::Occupied(mut o) => {
                      // Here is the output that the triggered register will cause to update
                      match program.mech.schedule.trigger_to_output.get(trigger_register) {
                        Some(output) => {
                          // Is any of this being listened for?
                          for (register,remote_cores) in &program.listeners {
                            if output.contains(&register) {
                              o.insert((register.clone(),remote_cores.clone()));
                              break;
                            }
                          }
                        }
                        None => ()
                      }
                      // We have listeners, so let's send them the changes
                      let ((output_table_id,row_ix,col_ix),listeners) = o.get();
                      let trigger = o.key();
                      match program.mech.get_table_by_id(*output_table_id.unwrap()) {
                        Ok(table) =>{
                          let table_brrw = table.borrow();
                          let changes = table_brrw.data_to_changes();
                          let message = bincode::serialize(&SocketMessage::Transaction(changes)).unwrap();
                          let compressed_message = compress_to_vec(&message,6);
                          // Send the transaction to the remote core
                          for remote_core_id in listeners {
                            match (&self.socket,program.remote_cores.get_mut(&remote_core_id)) {
                              (Some(ref socket),Some(MechSocket::UdpSocket(remote_core_address))) => {
                                let len = socket.send_to(&compressed_message, remote_core_address.clone()).unwrap();
                              }
                              (Some(ref socket),Some(MechSocket::WebSocketSender(websocket))) => {
                                match websocket.send_message(&OwnedMessage::Binary(compressed_message.clone())) {
                                  Ok(()) => (),
                                  Err(x) => {
                                    client_outgoing.send(ClientMessage::String(format!("Remote core disconnected: {}", humanize(&remote_core_id))));
                                    program.remote_cores.remove(&remote_core_id);
                                    for (core_id, core_address) in &program.remote_cores {
                                      match core_address {
                                        MechSocket::UdpSocket(core_address) => {
                                          let message = bincode::serialize(&SocketMessage::RemoteCoreDisconnect(*remote_core_id)).unwrap();
                                          let compressed_message = compress_to_vec(&message,6);
                                          let len = socket.send_to(&compressed_message, core_address.clone()).unwrap();
                                        }
                                        MechSocket::WebSocket(_) => {
                                          // TODO send disconnect message to websockets
                                        }
                                        _ => (),
                                      }
                                    }
                                  },
                                };
                              }
                              _ => (),
                            }
                          }
                        }
                        Err(MechError{id,kind}) => {
                          //client_outgoing.send(ClientMessage::Error(kind.clone()));
                        }
                      }
                    }
                    // Triggered for the first time
                    Entry::Vacant(mut v) => {
                      // Here is the output that the triggered register will cause to update
                      match program.mech.schedule.trigger_to_output.get(trigger_register) {
                        Some(output) => {
                          // Is any of this being listened for?
                          for (register,remote_cores) in &program.listeners {
                            if output.contains(&register) {
                              v.insert((register.clone(),remote_cores.clone()));
                              break;
                            }
                          }
                        }
                        None => ()
                      }
                    }
                  }
                }
              }
              Err(MechError{id,kind}) => {
                //client_outgoing.send(ClientMessage::Error(kind.clone()));
              }
            };
            let end_ns = time::precise_time_ns();
            let time = (end_ns - start_ns) as f64;
            client_outgoing.send(ClientMessage::Timing(1.0 / (time / 1_000_000_000.0)));
            client_outgoing.send(ClientMessage::StepDone);
          },
          (Ok(RunLoopMessage::Listening((core_id, register))), _) => {
            let (table_id,row,col) = &register;
            let name = program.mech.get_name(*table_id.unwrap()).unwrap();
            match program.mech.output.contains(&register.clone()) {
              // We produce a table for which they're listening
              true => {
                client_outgoing.send(ClientMessage::String(format!("Sending #{} to {}", name, humanize(&core_id))));
                // Mark down that this register has a listener for future updates
                let mut listeners = program.listeners.entry(register.clone()).or_insert(HashSet::new()); 
                listeners.insert(core_id);
                // Send over the table we have now
                match program.mech.get_table_by_id(*table_id.unwrap()) {
                  Ok(table) => {
                    // Decompose the table into changes for a transaction
                    let table_brrw = table.borrow();
                    let changes = table_brrw.to_changes();
                    let message = bincode::serialize(&SocketMessage::Transaction(changes)).unwrap();
                    let compressed_message = compress_to_vec(&message,6);
                    // Send the transaction to the remote core
                    match (&self.socket,program.remote_cores.get_mut(&core_id)) {
                      (Some(ref socket),Some(MechSocket::UdpSocket(remote_core_address))) => {
                        let len = socket.send_to(&compressed_message, remote_core_address.clone()).unwrap();
                      }
                      (_,Some(MechSocket::WebSocketSender(websocket))) => {
                        websocket.send_message(&OwnedMessage::Binary(compressed_message)).unwrap();
                      }
                      _ => (),
                    }
                  }
                  Err(_) => (),
                } 
              }, 
              false => (),
            }
          },
          (Ok(RunLoopMessage::RemoteCoreDisconnect(remote_core_id)), _) => {
            match &self.socket {
              Some(ref socket) => {
                let socket_address = hash_str(&socket.local_addr().unwrap().to_string());
                if remote_core_id != socket_address {
                  match program.remote_cores.get(&remote_core_id) {
                    None => {

                    } 
                    Some(_) => {
                      client_outgoing.send(ClientMessage::String(format!("Remote core disconnected: {}", humanize(&remote_core_id))));
                      program.remote_cores.remove(&remote_core_id);
                      for (core_id, core_address) in &program.remote_cores {
                        match core_address {
                          MechSocket::UdpSocket(core_address) => {
                            let message = bincode::serialize(&SocketMessage::RemoteCoreDisconnect(remote_core_id)).unwrap();
                            let compressed_message = compress_to_vec(&message,6);
                            let len = socket.send_to(&compressed_message, core_address.clone()).unwrap();
                          }
                          MechSocket::WebSocket(_) => {
                            // TODO send disconnect message to websockets
                          }
                          _ => (),
                        }
                      }
                    }
                  }
                }
              }
              None => (),
            }          
          }
          (Ok(RunLoopMessage::RemoteCoreConnect(MechSocket::UdpSocket(remote_core_address))), _) => {
            match &self.socket {
              Some(ref socket) => {
                let socket_address = socket.local_addr().unwrap().to_string();
                if remote_core_address != socket_address {
                  match program.remote_cores.get(&hash_str(&remote_core_address)) {
                    None => {
                      // We've got a new remote core. Let's ask it what it needs from us
                      // and tell it about all the other cores in our network.
                      program.remote_cores.insert(hash_str(&remote_core_address),MechSocket::UdpSocket(remote_core_address.clone()));
                      client_outgoing.send(ClientMessage::String(format!("Remote core connected: {}", humanize(&hash_str(&remote_core_address)))));
                      let message = bincode::serialize(&SocketMessage::RemoteCoreConnect(socket_address.clone())).unwrap();
                      let compressed_message = compress_to_vec(&message,6);                    
                      let len = socket.send_to(&compressed_message, remote_core_address.clone()).unwrap();
                      for (core_id, core_address) in &program.remote_cores {
                        match core_address {
                          MechSocket::UdpSocket(core_address) => {
                            let message = bincode::serialize(&SocketMessage::RemoteCoreConnect(core_address.to_string())).unwrap();
                            let compressed_message = compress_to_vec(&message,6);                    
                            let len = socket.send_to(&compressed_message, remote_core_address.clone()).unwrap();
                          }
                          MechSocket::WebSocket(_) => {
                            // TODO
                          }
                          _ => (),
                        }
                      }
                    } 
                    Some(_) => {
                      for register in &program.mech.needed_registers() {
                        //println!("I'm listening for {:?}", register);
                        let message = bincode::serialize(&SocketMessage::Listening(register.clone())).unwrap();
                        let compressed_message = compress_to_vec(&message,6);                    
                        let len = socket.send_to(&compressed_message, remote_core_address.clone()).unwrap();
                      }
                    }
                  }
                }
              }
              None => (),
            }
          } 
          (Ok(RunLoopMessage::RemoteCoreConnect(MechSocket::WebSocket(ws_stream))), _) => {
            let remote_core_address = ws_stream.peer_addr().unwrap();
            let remote_core_id = hash_str(&remote_core_address.to_string());
            let (mut ws_incoming, mut ws_outgoing) = ws_stream.split().unwrap();
            // Tell the remote websocket what this core is listening for
            for needed_register in &program.mech.needed_registers() {
              let message = bincode::serialize(&SocketMessage::Listening(needed_register.clone())).unwrap();
              let compressed_message = compress_to_vec(&message,6);
              ws_outgoing.send_message(&OwnedMessage::Binary(compressed_message)).unwrap();
            }
            // Store the websocket sender
            program.remote_cores.insert(remote_core_id, MechSocket::WebSocketSender(ws_outgoing));
            let program_channel_websocket = program.outgoing.clone();
            client_outgoing.send(ClientMessage::String(format!("Remote core connected: {}", humanize(&hash_str(&remote_core_address.to_string())))));
            thread::spawn(move || {
              for message in ws_incoming.incoming_messages() {
                let message = message.unwrap();
                match message {
                  OwnedMessage::Close(_) => {
                    return;
                  }
                  OwnedMessage::Binary(msg) => {
                    let message: Result<SocketMessage, bincode::Error> = bincode::deserialize(&msg);
                    match message {
                      Ok(SocketMessage::Listening(register)) => {
                        program_channel_websocket.send(RunLoopMessage::Listening((remote_core_id, register)));
                      }
                      Ok(SocketMessage::Transaction(txn)) => {
                        program_channel_websocket.send(RunLoopMessage::Transaction(txn));
                      },
                      x => {println!("Unhandled Message: {:?}", x);},
                    }
                  }
                  _ => (),
                }
              }  
            });
          }
          (Ok(RunLoopMessage::String((string,color))), _) => {
            let out_string = match color {
              Some(color) => {
                let r: u8 = (color >> 16) as u8;
                let g: u8 = (color >> 8) as u8;
                let b: u8 = color as u8;
                format!("{}", string.truecolor(r,g,b))
              },
              None => string,
            };
            client_outgoing.send(ClientMessage::String(out_string));
          } 
          (Ok(RunLoopMessage::Exit(exit_code)), _) => {
            client_outgoing.send(ClientMessage::Exit(exit_code));
          } 
          (Ok(RunLoopMessage::Code(code)), _) => {
            // Load the program
            let sections: Vec<Vec<SectionElement>> = match code {
              MechCode::String(code) => {
                let mut compiler = Compiler::new(); 
                match compiler.compile_str(&code) {
                  Ok(sections) => sections,
                  Err(err) => {
                    client_outgoing.send(ClientMessage::Error(err));
                    client_outgoing.send(ClientMessage::StepDone);
                    continue 'runloop;
                  }
                }
              },
              MechCode::MiniBlocks(mb_sections) => {
                let mut sections: Vec<Vec<SectionElement>> = vec![];
                for section in mb_sections {
                  let section: Vec<SectionElement> = section.iter().map(|mb| SectionElement::Block(MiniBlock::maximize_block(mb))).collect();
                  sections.push(section);
                } 
                sections
              }
            };

            let result = program.mech.load_sections(sections);

            for (new_block_ids,_,new_block_errors) in result {
              if new_block_errors.len() > 0 {
                match program.download_dependencies(Some(client_outgoing.clone())) {
                  Ok(resolved_errors) => {
                    let (_,_,nbo) = program.mech.resolve_errors(&resolved_errors);
                    program.mech.schedule_blocks();
                    for output in nbo {
                      program.mech.step(&output);
                    }
                  }
                  Err(err) => {
                    client_outgoing.send(ClientMessage::Error(err.clone()));
                  }
                }
              }

              if let Some(last_block_id) = new_block_ids.last() {
                let block = program.mech.blocks.get(last_block_id).unwrap().borrow();
                let out_id = match block.transformations.last() {
                  Some(Transformation::Function{name,arguments,out}) => {
                    let (out_id,_,_) = out;
                    *out_id
                  } 
                  Some(Transformation::TableDefine{table_id,indices,out}) => {
                    *out
                  } 
                  Some(Transformation::Set{src_id, src_row, src_col, dest_id, dest_row, dest_col}) => {
                    *dest_id
                  } 
                  Some(Transformation::TableAlias{table_id, alias}) => {
                    *table_id
                  } 
                  Some(Transformation::Whenever{table_id, ..}) => {
                    *table_id
                  } 
                  _ => {
                    TableId::Local(0)
                  }
                };
                if let Ok(out_table) = block.get_table(&out_id) {
                  client_outgoing.send(ClientMessage::String(format!("{:?}", out_table.borrow())));
                }
              }
            }

            // React to errors
            for (error,_) in program.mech.full_errors.iter() {
              client_outgoing.send(ClientMessage::Error(error.clone()));
            }
            client_outgoing.send(ClientMessage::StepDone);
          }
          (Ok(RunLoopMessage::Clear), _) => {
            /*program.clear();
            client_outgoing.send(ClientMessage::Clear);*/
          },
          (Ok(RunLoopMessage::PrintCore(core_id)), _) => {
            match core_id {
              None => client_outgoing.send(ClientMessage::String(format!("There are {:?} cores running.", program.cores.len() + 1))),
              Some(0) => client_outgoing.send(ClientMessage::String(format!("{:?}", program.mech))),
              Some(core_id) => client_outgoing.send(ClientMessage::String(format!("{:?}", program.cores.get(&core_id)))),
            };
          },
          (Ok(RunLoopMessage::PrintInfo), _) => {
            client_outgoing.send(ClientMessage::String(format!("{:?}", program)));
          },
          (Ok(RunLoopMessage::PrintTable(table_id)), _) => {
            let result = match program.mech.get_table_by_id(table_id) {
              Ok(table_ref) => format!("{:?}", table_ref.borrow()),
              Err(x) => format!("{:?}", x),
            };
            client_outgoing.send(ClientMessage::String(result));
          },
          (Ok(RunLoopMessage::Blocks(miniblocks)), _) => {
            /*let mut blocks: Vec<Block> = Vec::new();
            for miniblock in miniblocks {
              let mut block = Block::new(100);
              for tfms in miniblock.transformations {
                block.register_transformations(tfms);
              }
              blocks.push(block);
            }
            program.mech.register_blocks(blocks);
            program.mech.step();*/
            client_outgoing.send(ClientMessage::StepDone);
          }
          (Ok(RunLoopMessage::Stop), _) => { 
            client_outgoing.send(ClientMessage::Stop);
            break 'runloop;
          },
          (Ok(RunLoopMessage::GetValue((table_id,row,column))),_) => { 
            let msg = match program.mech.get_table_by_id(table_id) {
              Ok(table) => {
                match table.borrow().get(&row,&column) {
                  Ok(v) => ClientMessage::Value(v.clone()),
                  Err(error) => ClientMessage::Error(error.clone()),
                }
              }
              Err(error) => ClientMessage::Error(error.clone()),
            };
            client_outgoing.send(msg);
          },
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
          (Err(_), _) => {
            break 'runloop
          },
          x => println!("{:?}", x),
        }
        client_outgoing.send(ClientMessage::Done);
      }
      /*if let Some(channel) = persistence_channel {
        channel.send(PersisterMessage::Stop);
      }*/
    }).unwrap();

    Ok(RunLoop { name, socket_address, thread, outgoing: runloop_outgoing, incoming })
  }

  /*pub fn colored_name(&self) -> term_painter::Painted<String> {
    BrightCyan.paint(format!("[{}]", &self.name))
  }*/

}
