// # Program

// # Prelude
extern crate bincode;
extern crate libloading;
extern crate colored;
use colored::*;

use std::thread::{self, JoinHandle};
use std::collections::{HashMap, HashSet, Bound, BTreeMap};
use std::collections::hash_map::Entry;
use std::mem;
use std::fs::{OpenOptions, File, canonicalize, create_dir};
use std::io::{Write, BufReader, BufWriter};
use std::sync::Arc;
use std::rc::Rc;
use std::path::{Path, PathBuf};

use mech_core::{Core, Register, Transaction, Change, Error};
use mech_core::{Value, Index};
use mech_core::Block;
use mech_core::{Table, TableIndex, Hasher, TableId};
use mech_syntax::compiler::Compiler;
use mech_utilities::{RunLoopMessage, Watcher};
use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;

use libloading::Library;
use std::io::copy;

use time;

fn download_machine(machine_name: &str, name: &str, path_str: &str, ver: &str, outgoing: Option<crossbeam_channel::Sender<ClientMessage>>) -> Result<Library,Box<std::error::Error>> {
  create_dir("machines");

  let machine_file_path = format!("machines/{}",machine_name);
  {
    let path = Path::new(path_str);
    // Download from the web
    if path.to_str().unwrap().starts_with("https") {
      match outgoing {
        Some(sender) => {sender.send(ClientMessage::String(format!("{} {} v{}", "[Downloading]".bright_cyan(), name, ver)));}
        None => (),
      }
      let machine_url = format!("{}/{}", path_str, machine_name);
      let mut response = reqwest::get(machine_url.as_str())?;
      let mut dest = File::create(machine_file_path.clone())?;
      copy(&mut response, &mut dest)?;
    // Load from a local directory
    } else {
      match outgoing {
        Some(sender) => {sender.send(ClientMessage::String(format!("{} {} v{}", "[Loading]".bright_cyan(), name, ver)));}
        None => (),
      }
      let machine_path = format!("{}{}", path_str, machine_name);
      println!("{:?}", machine_path);
      let path = Path::new(&machine_path);
      let mut dest = File::create(machine_file_path.clone())?;
      let mut f = File::open(path)?;
      copy(&mut f, &mut dest)?;
    }
  }
  let machine_file_path = format!("machines/{}",machine_name);
  let machine = Library::new(machine_file_path).expect("Can't load library");
  Ok(machine)
}

// ## Program

pub struct Program {
  pub name: String,
  pub mech: Core,
  pub cores: HashMap<u64,Core>,
  pub input_map: HashMap<Register,HashSet<u64>>,
  pub machines: HashMap<String, Library>,
  pub watchers: HashMap<u64, Box<Watcher + Send>>,
  pub machine_registry: HashMap<String, (String, String)>,
  capacity: usize,
  pub incoming: Receiver<RunLoopMessage>,
  pub outgoing: Sender<RunLoopMessage>,
  pub errors: Vec<Error>,
  programs: u64,
  pub listeners: HashSet<TableId>,
}

impl Program {
  pub fn new(name:&str, capacity: usize, outgoing: Sender<RunLoopMessage>, incoming: Receiver<RunLoopMessage>) -> Program {
    let (outgoing, incoming) = crossbeam_channel::unbounded();
    let mut mech = Core::new(capacity, 100);
    let mech_code = Hasher::hash_str("mech/code");
    let txn = Transaction::from_change(Change::NewTable{id: mech_code, rows: 1, columns: 1});
    mech.process_transaction(&txn);
    Program { 
      name: name.to_owned(), 
      watchers: HashMap::new(),
      capacity,
      machine_registry: HashMap::new(), 
      mech,
      cores: HashMap::new(),
      machines: HashMap::new(),
      input_map: HashMap::new(),
      incoming,
      outgoing,
      errors: Vec::new(),
      programs: 0,
      listeners: HashSet::new(),
    }
  }

  pub fn compile_program(&mut self, input: String) {
    let mut compiler = Compiler::new();
    compiler.compile_string(input.clone());
    self.mech.register_blocks(compiler.blocks);
    self.errors.append(&mut self.mech.runtime.errors.clone());
    let mech_code = Hasher::hash_str("mech/code");
    self.programs += 1;
    let txn = Transaction::from_change(Change::Set{table: mech_code, row: Index::Index(self.programs), column: Index::Index(1), value: Value::from_str(&input.clone())});
    //self.outgoing.send(RunLoopMessage::Transaction(txn));
  }

  pub fn compile_fragment(&mut self, input: String) {
    let mut compiler = Compiler::new();
    compiler.compile_string(input.clone());
    for mut block in compiler.blocks {
      block.id = self.mech.runtime.blocks.len() + 1;
      self.mech.runtime.ready_blocks.insert(block.id);
      self.mech.register_blocks(vec![block]);
    }
    self.errors.append(&mut self.mech.runtime.errors.clone());
    let mech_code = Hasher::hash_str("mech/code");
    self.programs += 1;
    let txn = Transaction::from_change(Change::Set{table: mech_code, row: Index::Index(self.programs), column: Index::Index(1), value: Value::from_str(&input.clone())});
    //self.outgoing.send(RunLoopMessage::Transaction(txn));
  }

  pub fn download_dependencies(&mut self, outgoing: Option<crossbeam_channel::Sender<ClientMessage>>) -> Result<(),Box<std::error::Error>> {

    if self.machine_registry.len() == 0 {
      // Download repository index
      let registry_url = "https://gitlab.com/mech-lang/machines/-/raw/master/machines.mec";
      let mut response = reqwest::get(registry_url)?.text()?;
      let mut registry_compiler = Compiler::new();
      registry_compiler.compile_string(response);
      let mut registry_core = Core::new(1,1);
      registry_core.register_blocks(registry_compiler.blocks);
      registry_core.step();

      // Convert the machine listing into a hash map
      let registry_table = registry_core.get_table("mech/machines".to_string()).unwrap().borrow();
      for row in 0..registry_table.rows {
        let row_index = Index::Index(row+1);
        let name = registry_table.index(&row_index, &Index::Index(1)).unwrap().as_string().unwrap();
        let version = registry_table.index(&row_index, &Index::Index(2)).unwrap().as_string().unwrap();
        let url = registry_table.index(&row_index, &Index::Index(3)).unwrap().as_string().unwrap();
        self.machine_registry.insert(name, (version, url));
      }
    }

    // Do it for the mech core
    for (fun_name, fun) in self.mech.runtime.functions.iter_mut() {
      let m: Vec<_> = fun_name.split('/').collect();
      #[cfg(unix)]
      let machine_name = format!("libmech_{}.so", m[0]);
      #[cfg(windows)]
      let machine_name = format!("mech_{}.dll", m[0]);
      match (&fun, self.machine_registry.get(m[0])) {
        (None, Some((ver, path))) => {
          let machine = self.machines.entry(m[0].to_string()).or_insert_with(||{
            match File::open(format!("machines/{}",machine_name)) {
              Ok(_) => {
                Library::new(format!("machines/{}",machine_name)).expect("Can't load library")
              }
              _ => download_machine(&machine_name, m[0], path, ver, outgoing.clone()).unwrap()
            }
          });       
          let native_rust = unsafe {
            // Replace slashes with underscores and then add a null terminator
            let mut s = format!("{}\0", fun_name.replace("/","_"));
            let error_msg = format!("Symbol {} not found",s);
            let m = machine.get::<extern "C" fn(Vec<(String, Table)>)->Table>(s.as_bytes()).expect(&error_msg);
            m.into_raw()
          };
          *fun = Some(*native_rust);
        },
        _ => (),
      }
    }
    
    // Do it for the the other core
    for core in self.cores.values_mut() {
      for (fun_name, fun) in core.runtime.functions.iter_mut() {
        let m: Vec<_> = fun_name.split('/').collect();
        #[cfg(unix)]
        let machine_name = format!("libmech_{}.so", m[0]);
        #[cfg(windows)]
        let machine_name = format!("mech_{}.dll", m[0]);
        match (&fun, self.machine_registry.get(m[0])) {
          (None, Some((ver, path))) => {
  
            let machine = self.machines.entry(m[0].to_string()).or_insert_with(||{
              match File::open(format!("machines/{}",machine_name)) {
                Ok(_) => {
                  Library::new(format!("machines/{}",machine_name)).expect("Can't load library")
                }
                _ => download_machine(&machine_name, m[0], path, ver, outgoing.clone()).unwrap()
              }
            });          
            let native_rust = unsafe {
              // Replace slashes with underscores and then add a null terminator
              let mut s = format!("{}\0", fun_name.replace("/","_"));
              let error_msg = format!("Symbol {} not found",s);
              let m = machine.get::<extern "C" fn(Vec<(String, Table)>)->Table>(s.as_bytes()).expect(&error_msg);
              m.into_raw()
            };
            *fun = Some(*native_rust);
          },
          _ => (),
        }
      }
    }
    
    Ok(())
  }

  pub fn clear(&mut self) {
    self.mech.clear();
  }

}

// ## Run Loop

// Client messages are sent to the client from the run loop

#[derive(Debug, Clone)]
pub enum ClientMessage {
  Stop,
  Pause,
  Resume,
  Clear,
  Time(usize),
  NewBlocks(usize),
  Table(Option<Table>),
  Transaction(Transaction),
  String(String),
  //Block(Block),
  Done,
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

  pub fn channel(&self) -> Sender<RunLoopMessage> {
    self.outgoing.clone()
  }

}

// ## Persister

pub enum PersisterMessage {
    Stop,
    Write(Vec<Change>),
}

pub struct Persister {
    thread: JoinHandle<()>,
    outgoing: Sender<PersisterMessage>,
    loaded: Vec<Change>,
}

impl Persister {
  pub fn new(path_ref:&str) -> Persister {
    let (outgoing, incoming) = crossbeam_channel::unbounded();
    let path = path_ref.to_string();
    let thread = thread::spawn(move || {
      let file = OpenOptions::new().append(true).create(true).open(&path).unwrap();
      let mut writer = BufWriter::new(file);
      loop {
        match incoming.recv().unwrap() {
          PersisterMessage::Stop => { break; }
          PersisterMessage::Write(items) => {
            for item in items {
              let result = bincode::serialize(&item).unwrap();
              if let Err(e) = writer.write_all(&result) {
                panic!("Can't persist! {:?}", e);
              }
            }
            writer.flush().unwrap();
          }
        }
      }
    });
    Persister { outgoing, thread, loaded: vec![] }
  }

  pub fn load(&mut self, path: &str) {
    let file = match File::open(path) {
      Ok(f) => f,
      Err(_) => {
        ////println!("Unable to load db: {}", path);
        return;
      }
    };
    let mut reader = BufReader::new(file);
    loop {
      let result:Result<Change, _> = bincode::deserialize_from(&mut reader);
      match result {
        Ok(change) => {
          self.loaded.push(change);
        },
        Err(info) => {
          ////println!("ran out {:?}", info);
          break;
        }
      }
    }
  }

  pub fn send(&self, changes: Vec<Change>) {
    self.outgoing.send(PersisterMessage::Write(changes)).unwrap();
  }

  pub fn wait(self) {
    self.thread.join().unwrap();
  }

  pub fn get_channel(&self) -> Sender<PersisterMessage> {
    self.outgoing.clone()
  }

  pub fn get_changes(&mut self) -> Vec<Change> {
    mem::replace(&mut self.loaded, vec![])
  }

  pub fn close(&self) {
    self.outgoing.send(PersisterMessage::Stop).unwrap();
  }
}
// ## Program Runner

pub struct ProgramRunner {
  pub name: String,
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

  pub fn attach_watcher(&mut self, watcher:Box<Watcher + Send>) {
    //let name = Hasher::hash_str(&watcher.get_name());
    //let columns = watcher.get_columns().clone() as u64;
    //self.program.watchers.insert(name, watcher);
    //let watcher_table = Transaction::from_change(Change::NewTable{id: name, rows: 1, columns});
    //self.program.outgoing.send(RunLoopMessage::Transaction(watcher_table));
  }

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

      let mut program = Program::new("new program", 100, outgoing.clone(), program_incoming);

      program.download_dependencies(Some(client_outgoing.clone()));

      // Step cores
      program.mech.step();
      for core in program.cores.values_mut() {
        core.step();
      }

      // Send the first done to the client to indicate that the program is initialized
      client_outgoing.send(ClientMessage::Done);
      println!("Sent a done message");
      let mut paused = false;
      'runloop: loop {
        println!("In the run loop");
        match (program.incoming.recv(), paused) {
          (Ok(RunLoopMessage::Transaction(txn)), false) => {
            //println!("{} Txn started:\n {:?}", name, txn);
            let pre_changes = program.mech.store.len();
            let start_ns = time::precise_time_ns();
            program.mech.process_transaction(&txn);
            let delta_changes = program.mech.store.len() - pre_changes;
            let end_ns = time::precise_time_ns();
            let time = (end_ns - start_ns) as f64;              
            //program.compile_string(String::from(text.clone()));
            ////println!("{:?}", program.mech);
            ////println!("{} Txn took {:0.4?} ms ({:0.0?} cps)", name, time / 1_000_000.0, delta_changes as f64 / (time / 1.0e9));
            let mut changes: Vec<Change> = Vec::new();
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
            }
          },
          (Ok(RunLoopMessage::Listening(table_ids)), _) => {
            for table_id in table_ids {
              match program.mech.output.get(&Register::new(table_id, Index::Index(0))) {
                Some(_) => {program.listeners.insert(table_id);}, // We produce a table for which they're listening, so let's mark that
                _ => (),
              }
            }
          },
          (Ok(RunLoopMessage::Stop), _) => { 
            client_outgoing.send(ClientMessage::Stop);
            break 'runloop;
          },
          (Ok(RunLoopMessage::Table(table_id)), _) => { 
            /*let table_msg = match program.mech.store.get_table(table_id) {
              Some(table) => ClientMessage::Table(Some(table.clone())),
              None => ClientMessage::Table(None),
            };
            client_outgoing.send(table_msg);*/
          },
          (Ok(RunLoopMessage::Pause), false) => { 
            paused = true;
            client_outgoing.send(ClientMessage::Pause);
          },
          (Ok(RunLoopMessage::Resume), true) => {
            paused = false;
            program.mech.resume();
            client_outgoing.send(ClientMessage::Resume);
          },
          (Ok(RunLoopMessage::StepBack), _) => {
            if !paused {
              paused = true;
            }
            program.mech.step_back_one();
            client_outgoing.send(ClientMessage::Time(program.mech.offset));
          }
          (Ok(RunLoopMessage::StepForward), true) => {
            program.mech.step_forward_one();
            client_outgoing.send(ClientMessage::Time(program.mech.offset));
          } 
          (Ok(RunLoopMessage::Code(code)), _) => {
            println!("Loading code");
            let block_count = program.mech.runtime.blocks.len();
            program.compile_fragment(code);
            program.download_dependencies(Some(client_outgoing.clone()));
            program.mech.step();
          }
          (Ok(RunLoopMessage::EchoCode(code)), _) => {
            println!("ECHO CODE!! {:?}", code);
            
            let block_count = program.mech.runtime.blocks.len();
            program.compile_fragment(code);

            
            program.download_dependencies(Some(client_outgoing.clone()));

            println!("{:?}", program.mech);
            for core in &program.cores {
              println!("{:?}", core);
            }


            program.mech.step();
            

            let echo_table = match program.mech.get_table("ans".to_string()) {
              Some(table) => Some(table.clone()),
              None => None,
            };
            //client_outgoing.send(ClientMessage::Table(echo_table));
          } 
          (Ok(RunLoopMessage::Clear), _) => {
            program.clear();
            client_outgoing.send(ClientMessage::Clear);
          },
          (Ok(RunLoopMessage::PrintCore(core_id)), _) => {
            match core_id {
              None => client_outgoing.send(ClientMessage::String(format!("{:?}",program.cores.len() + 1))),
              Some(0) => client_outgoing.send(ClientMessage::String(format!("{:?}",program.mech))),
              Some(core_id) => client_outgoing.send(ClientMessage::String(format!("{:?}",program.cores.get(&core_id)))),
            };
          },
          (Ok(RunLoopMessage::PrintRuntime), _) => {
            client_outgoing.send(ClientMessage::String(format!("{:?}",program.mech.runtime)));
          },
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
    RunLoop { name: self.name, thread, outgoing: runloop_outgoing, incoming }
  }

  /*pub fn colored_name(&self) -> term_painter::Painted<String> {
    BrightCyan.paint(format!("[{}]", &self.name))
  }*/

}
