// # Program

// # Prelude
extern crate bincode;
extern crate libloading;

use std::sync::mpsc::{Sender, Receiver, SendError};
use std::thread::{self, JoinHandle};
use std::sync::mpsc;
use std::collections::{HashMap, HashSet, Bound, BTreeMap};
use std::collections::hash_map::Entry;
use std::mem;
use std::fs::{OpenOptions, File, canonicalize, create_dir};
use std::io::{Write, BufReader, BufWriter};
use std::sync::Arc;

use mech_core::{Core, Register, Transaction, Change, Error};
use mech_core::{Value, Index};
use mech_core::Block;
use mech_core::{Table, TableIndex, Hasher, TableId};
use mech_syntax::compiler::Compiler;
use mech_utilities::{RunLoopMessage, Watcher};

use libloading::Library;
use std::io::copy;

use time;

// ## Program

pub struct Program {
  pub name: String,
  pub mech: Core,
  pub watchers: HashMap<u64, Box<Watcher + Send>>,
  capacity: usize,
  pub incoming: Receiver<RunLoopMessage>,
  pub outgoing: Sender<RunLoopMessage>,
  pub errors: Vec<Error>,
  programs: u64,
  pub listeners: HashSet<TableId>,
}

impl Program {
  pub fn new(name:&str, capacity: usize) -> Program {
    let (outgoing, incoming) = mpsc::channel();
    let mut mech = Core::new(capacity, 100);
    let mech_code = Hasher::hash_str("mech/code");
    let txn = Transaction::from_change(Change::NewTable{id: mech_code, rows: 1, columns: 1});
    mech.process_transaction(&txn);
    Program { 
      name: name.to_owned(), 
      watchers: HashMap::new(),
      capacity,
      mech,
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
    self.outgoing.send(RunLoopMessage::Transaction(txn));
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
    self.outgoing.send(RunLoopMessage::Transaction(txn));
    self.mech.step();
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
  Block(Block),
  Done,
}

pub struct RunLoop {
  thread: JoinHandle<()>,
  outgoing: Sender<RunLoopMessage>,
  incoming: Receiver<ClientMessage>,
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
    let (outgoing, incoming) = mpsc::channel();
    let path = path_ref.to_string();
    let thread = thread::spawn(move || {
      let file = OpenOptions::new().append(true).create(true).open(&path).unwrap();
      let mut writer = BufWriter::new(file);
      loop {
        match incoming.recv().unwrap() {
          PersisterMessage::Stop => { break; }
          PersisterMessage::Write(items) => {
            for item in items {
              let result = bincode::serialize(&item, bincode::Infinite).unwrap();
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
      let result:Result<Change, _> = bincode::deserialize_from(&mut reader, bincode::Infinite);
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
  pub program: Program, 
  pub mechanisms: HashMap<String, Library>,
  pub persistence_channel: Option<Sender<PersisterMessage>>,
}

impl ProgramRunner {

  pub fn new(name:&str, capacity: usize) -> ProgramRunner {
    // Start a new program
    let mut program = Program::new(name, capacity);

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
      program,
      mechanisms: HashMap::new(),
      // TODO Use the persistence file specified by the user
      //persistence_channel: Some(persister.get_channel()),
      persistence_channel: None,
    }
  }

  pub fn load_program(&mut self, input: String) -> Result<(),Box<std::error::Error>> {
    self.program.compile_program(input);

    let mut registry: HashMap<&str, (&str, &str)> = HashMap::new();
    registry.insert("math", ("0.0.1", "https://github.com/mech-lang/math/releases/download/v0.0.1/"));
    registry.insert("stat", ("0.0.1", "https://github.com/mech-lang/stat/releases/download/v0.0.1/"));

    for (fun_name, fun) in self.program.mech.runtime.functions.iter_mut() {
      let m: Vec<_> = fun_name.split('/').collect();
      match (&fun, registry.get(m[0])) {
        (None, Some((ver, path))) => {

          fn download_mism(name: &str, path: &str, ver: &str) -> Result<Library,Box<std::error::Error>> {
            create_dir("misms");

            #[cfg(unix)]
            let mism_name = format!("libmech_{}.so", name);
            #[cfg(windows)]
            let mism_name = format!("mech_{}.dll", name);
            let mism_file_path = format!("misms/{}",mism_name);
            // TODO Do path and URL. Right now, assume URL
            {
              println!("Downloading: {} v{}", name, ver);
              let mism_url = format!("{}{}", path, mism_name);
              let mut response = reqwest::get(mism_url.as_str())?;
              let mut dest = File::create(mism_file_path.clone())?;
              copy(&mut response, &mut dest)?;
            }
            let mism_file_path = format!("misms/{}",mism_name);
            let mism = Library::new(mism_file_path).expect("Can't load library");
            Ok(mism)
          }

          let mechanism = self.mechanisms.entry(m[0].to_string()).or_insert_with(||{
            download_mism(m[0], path, ver).unwrap()
          });       
          let native_rust = unsafe {
            // Replace slashes with underscores and then add a null terminator
            let mut s = format!("{}\0", fun_name.replace("/","_"));
            let error_msg = format!("Symbol {} not found",s);
            let m = mechanism.get::<extern "C" fn(Vec<(String, Table)>)->Table>(s.as_bytes()).expect(&error_msg);
            m.into_raw()
          };
          *fun = Some(*native_rust);
        },
        _ => (),
      }
    }
    self.program.mech.step();
    Ok(())
  }

  pub fn attach_watcher(&mut self, watcher:Box<Watcher + Send>) {
    //let name = Hasher::hash_str(&watcher.get_name());
    //let columns = watcher.get_columns().clone() as u64;
    //self.program.watchers.insert(name, watcher);
    //let watcher_table = Transaction::from_change(Change::NewTable{id: name, rows: 1, columns});
    //self.program.outgoing.send(RunLoopMessage::Transaction(watcher_table));
  }

  pub fn add_persist_channel(&mut self, persister:&mut Persister) {
    self.persistence_channel = Some(persister.get_channel());
  }

  pub fn run(self) -> RunLoop {
    let name = self.name;
    let outgoing = self.program.outgoing.clone();
    let (client_outgoing, incoming) = mpsc::channel();
    let mut program = self.program;
    let persistence_channel = self.persistence_channel;
    let thread = thread::Builder::new().name(program.name.to_owned()).spawn(move || {
      let mut paused = false;
      'runloop: loop {
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
            client_outgoing.send(ClientMessage::Done);
          },
          (Ok(RunLoopMessage::Stop), _) => { 
            client_outgoing.send(ClientMessage::Stop);
            break 'runloop;
          },
          (Ok(RunLoopMessage::Table(table_id)), _) => { 
            let table_msg = match program.mech.store.get_table(table_id) {
              Some(table) => ClientMessage::Table(Some(table.clone())),
              None => ClientMessage::Table(None),
            };
            client_outgoing.send(table_msg);
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
            let block_count = program.mech.runtime.blocks.len();
            program.compile_fragment(code);
            client_outgoing.send(ClientMessage::Done);
          } 
          (Ok(RunLoopMessage::Clear), _) => {
            program.clear();
            client_outgoing.send(ClientMessage::Clear);
          },
          (Ok(RunLoopMessage::PrintCore), _) => {
            client_outgoing.send(ClientMessage::String(format!("{:?}",program.mech)));
          },
          (Ok(RunLoopMessage::PrintRuntime), _) => {
            client_outgoing.send(ClientMessage::String(format!("{:?}",program.mech.runtime)));
          },
          (Err(_), _) => break 'runloop,
          _ => (),
        }
      }
      if let Some(channel) = persistence_channel {
        channel.send(PersisterMessage::Stop);
      }
    }).unwrap();
    RunLoop { thread, outgoing, incoming }
  }

  /*pub fn colored_name(&self) -> term_painter::Painted<String> {
    BrightCyan.paint(format!("[{}]", &self.name))
  }*/

}
