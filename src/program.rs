// # Program

// # Prelude
extern crate bincode;

use std::sync::mpsc::{Sender, Receiver, SendError};
use std::thread::{self, JoinHandle};
use std::sync::mpsc;
use std::collections::{HashMap, HashSet, Bound, BTreeMap};
use std::mem;
use std::fs::{OpenOptions, File, canonicalize};
use std::io::{Write, BufReader, BufWriter};

use mech_core::{Core, Transaction, Change, Error};
use mech_core::{Value, Index};
use mech_core::Block;
use mech_core::{TableIndex, Hasher};
use mech_syntax::compiler::Compiler;
use super::Watcher;

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
    }
  }

  pub fn compile_string(&mut self, input: String) {
    let mut compiler = Compiler::new();
    compiler.compile_string(input.clone());
    self.mech.register_blocks(compiler.blocks);
    self.errors.append(&mut self.mech.runtime.errors.clone());
    let mech_code = Hasher::hash_str("mech/code");
    let txn = Transaction::from_change(Change::Set{table: mech_code, row: Index::Index(1), column: Index::Index(1), value: Value::from_str(&input.clone())});
    self.outgoing.send(RunLoopMessage::Transaction(txn));
    self.mech.step();
  }

  pub fn clear(&mut self) {
    self.mech.clear();
  }

}

// ## Run Loop

#[derive(Debug, Clone)]
pub enum RunLoopMessage {
  Stop,
  StepBack,
  StepForward,
  Pause,
  Resume,
  Clear,
  Transaction(Transaction),
  Code(String),
}

pub struct RunLoop {
  thread: JoinHandle<()>,
  outgoing: Sender<RunLoopMessage>,
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
        println!("Unable to load db: {}", path);
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
          println!("ran out {:?}", info);
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
  pub persistence_channel: Option<Sender<PersisterMessage>>,
}

impl ProgramRunner {

  pub fn new(name:&str, capacity: usize) -> ProgramRunner {
    // Start a new program
    let mut program = Program::new(name, capacity);

    // Start a persister
    let persist_name = format!("{}.mdb", name);
    let mut persister = Persister::new(&persist_name);
    persister.load(&persist_name);
    let changes = persister.get_changes();

    // Load database
    //println!("{} Applying {} stored changes...", BrightCyan.paint(format!("[{}]", name)), changes.len());    
    for change in changes {
      program.mech.process_transaction(&Transaction::from_change(change));
    }
    
    ProgramRunner {
      name: name.to_owned(),
      program,
      // TODO Use the persistence file specified by the user
      persistence_channel: Some(persister.get_channel()),
    }
  }

  pub fn load_program(&mut self, input: String) {
    self.program.compile_string(input);
  }

  pub fn attach_watcher(&mut self, watcher:Box<Watcher + Send>) {
    let name = Hasher::hash_str(&watcher.get_name());
    let columns = watcher.get_columns().clone() as u64;
    self.program.watchers.insert(name, watcher);
    let watcher_table = Transaction::from_change(Change::NewTable{id: name, rows: 1, columns});
    self.program.outgoing.send(RunLoopMessage::Transaction(watcher_table));
  }

  pub fn add_persist_channel(&mut self, persister:&mut Persister) {
    self.persistence_channel = Some(persister.get_channel());
  }

  pub fn run(self) -> RunLoop {
    let name = self.name;
    let outgoing = self.program.outgoing.clone();
    let mut program = self.program;
    let persistence_channel = self.persistence_channel;
    let thread = thread::Builder::new().name(program.name.to_owned()).spawn(move || {
      println!("{} Starting run loop.", name);
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
            //println!("{:?}", program.mech);
            //println!("{} Txn took {:0.4?} ms ({:0.0?} cps)", name, time / 1_000_000.0, delta_changes as f64 / (time / 1.0e9));
          },
          (Ok(RunLoopMessage::Stop), _) => { 
            break 'runloop;
          },
          (Ok(RunLoopMessage::Pause), false) => { 
            paused = true;
            println!("{} Run loop paused.", name);            
          },
          (Ok(RunLoopMessage::Resume), true) => {
            paused = false;
            println!("{} Run loop resumed.", name);
            program.mech.resume();
          },
          (Ok(RunLoopMessage::StepBack), _) => {
            if !paused {
              paused = true;
              println!("{} Run loop paused.", name);
            }
            program.mech.step_back_one();
          }
          (Ok(RunLoopMessage::StepForward), true) => {
            program.mech.step_forward_one();
          } 
          (Ok(RunLoopMessage::Code(code)), _) => {
            println!("{} Loading code\n{:?}", name, code);
            program.clear();
            program.compile_string(code);
            println!("{:?}", program.mech.runtime);
          } 
          (Ok(RunLoopMessage::Clear), _) => {
            println!("{} Clearing program.", name);
            program.clear()
          },
          (Err(_), _) => break 'runloop,
          _ => (),
        }
      }
      if let Some(channel) = persistence_channel {
        channel.send(PersisterMessage::Stop);
      }
      println!("{} Run loop closed.", name);
    }).unwrap();
    RunLoop { thread, outgoing }
  }

  /*pub fn colored_name(&self) -> term_painter::Painted<String> {
    BrightCyan.paint(format!("[{}]", &self.name))
  }*/

}
