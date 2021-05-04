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
use std::cell::RefCell;
use std::path::{Path, PathBuf};

use mech_core::{Core, humanize, Register, Transaction, Change, Error, ErrorType};
use mech_core::{Value, ValueMethods, ValueIterator, TableIndex};
use mech_core::{Block, BlockState};
use mech_core::{Table, TableId};
use mech_core::hash_string;
use mech_syntax::compiler::Compiler;
use mech_utilities::{RunLoopMessage, MechCode, Machine, MachineRegistrar, MachineDeclaration};
use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;

use super::download_machine;
use super::persister::Persister;
use super::runloop::ClientMessage;

use libloading::Library;
use std::io::copy;

use time;

lazy_static! {
  static ref MECH_CODE: u64 = hash_string("mech/code");
  static ref MECH_MACHINES: u64 = hash_string("mech/machines");
  static ref NAME: u64 = hash_string("name");
  static ref VERSION: u64 = hash_string("version");
  static ref URL: u64 = hash_string("url");
}


struct Registrar {
  machines: HashMap<u64, Box<dyn Machine>>,
}

impl Registrar {
  fn new() -> Registrar {
    Registrar {
      machines: HashMap::default(),
    }
  }
}

impl MachineRegistrar for Registrar {
  fn register_machine(&mut self, machine: Box<dyn Machine>) {
    self.machines.insert(machine.id(), machine);
  }
}

// ## Program

pub struct Program {
  pub name: String,
  pub mech: Core,
  pub cores: HashMap<u64,Core>,
  pub input_map: HashMap<Register,HashSet<u64>>,
  pub libraries: HashMap<String, Library>,
  pub machines: HashMap<u64, Box<dyn Machine>>,
  pub machine_repository: HashMap<String, (String, String)>,
  capacity: usize,
  pub incoming: Receiver<RunLoopMessage>,
  pub outgoing: Sender<RunLoopMessage>,
  pub errors: Vec<Error>,
  programs: u64,
  loaded_machines: HashSet<u64>,
  pub listeners: HashSet<Register>,
}

impl Program {
  pub fn new(name:&str, capacity: usize, recursion_limit: u64, outgoing: Sender<RunLoopMessage>, incoming: Receiver<RunLoopMessage>) -> Program {
    let mut mech = Core::new(capacity, recursion_limit);
    mech.load_standard_library();
    let mech_code = hash_string("mech/code");
    let txn = Transaction{changes: vec![Change::NewTable{table_id: mech_code, rows: 1, columns: 1}]};
    mech.process_transaction(&txn);
    Program { 
      name: name.to_owned(), 
      capacity,
      machine_repository: HashMap::new(), 
      mech,
      cores: HashMap::new(),
      libraries: HashMap::new(),
      machines: HashMap::new(),
      loaded_machines: HashSet::new(),
      input_map: HashMap::new(),
      incoming,
      outgoing,
      errors: Vec::new(),
      programs: 0,
      listeners: HashSet::new(),
    }
  }

  pub fn trigger_machines(&mut self) {
    let database = self.mech.runtime.database.borrow();
    for register in &self.mech.runtime.aggregate_changed_this_round {
      match self.machines.get_mut(&register.hash()) {
        // Invoke the machine!
        Some(mut machine) => {
          let table = database.tables.get(&register.table_id.unwrap()).unwrap();
          machine.on_change(&table);
        },
        _ => (), // TODO Warn user that the machine is not loaded!
      }
    }
  }

  pub fn compile_program(&mut self, input: String) {
    let mut compiler = Compiler::new();
    compiler.compile_string(input.clone());
    self.mech.register_blocks(compiler.blocks);
    //self.errors.append(&mut self.mech.runtime.errors.clone());
    let mech_code = *MECH_CODE;
    self.programs += 1;
    //let txn = Transaction::from_change(Change::Set{table: mech_code, row: TableIndex::Index(self.programs), column: TableIndex::Index(1), value: Value::from_str(&input.clone())});
    //self.outgoing.send(RunLoopMessage::Transaction(txn));
  }

  pub fn compile_fragment(&mut self, input: String) {
    let mut compiler = Compiler::new();
    compiler.compile_string(input.clone());
    for mut block in compiler.blocks {
      block.id = (self.mech.runtime.blocks.len() + 1) as u64;
      self.mech.runtime.ready_blocks.insert(block.id);
      self.mech.register_blocks(vec![block]);
    }
    //self.errors.append(&mut self.mech.runtime.errors.clone());
    let mech_code = *MECH_CODE;
    self.programs += 1;
    //let txn = Transaction::from_change(Change::Set{table: mech_code, row: TableIndex::Index(self.programs), column: TableIndex::Index(1), value: Value::from_str(&input.clone())});
    //self.outgoing.send(RunLoopMessage::Transaction(txn));
  }

  pub fn download_dependencies(&mut self, outgoing: Option<crossbeam_channel::Sender<ClientMessage>>) -> Result<(),Box<std::error::Error>> {
    if self.machine_repository.len() == 0 {
      // Download machine_repository index
      let registry_url = "https://gitlab.com/mech-lang/machines/directory/-/raw/main/machines.mec";
      let mut response = reqwest::get(registry_url)?.text()?;
      let mut registry_compiler = Compiler::new();
      registry_compiler.compile_string(response);
      let mut registry_core = Core::new(100,100);
      registry_core.load_standard_library(); 
      registry_core.register_blocks(registry_compiler.blocks);
      registry_core.step();

      // Convert the machine listing into a hash map
      let registry_table = registry_core.get_table(*MECH_MACHINES).unwrap();
      for row in 0..registry_table.rows {
        let row_index = TableIndex::Index(row+1);
        let (name,_) = registry_table.get_string(&row_index, &TableIndex::Alias(*NAME)).unwrap();
        let (version,_) = registry_table.get_string(&row_index, &TableIndex::Alias(*VERSION)).unwrap();
        let (url,_) = registry_table.get_string(&row_index, &TableIndex::Alias(*URL)).unwrap();
        self.machine_repository.insert(name.to_string(), (version.to_string(), url.to_string()));
      }
    }
    // Do it for the mech core
    for (fun_name_id, fun) in self.mech.runtime.functions.iter_mut() {
      let fun_name = self.mech.runtime.database.borrow().store.strings.get(&fun_name_id).unwrap().clone();
      let m: Vec<_> = fun_name.split('/').collect();
      let m = m[0];
      let underscore_name = m.replace("-","_");
      #[cfg(unix)]
      let machine_name = format!("libmech_{}.so", underscore_name);
      #[cfg(windows)]
      let machine_name = format!("mech_{}.dll", underscore_name);

      match (&fun, self.machine_repository.get(m)) {
        (None, Some((ver, path))) => {
          let library = self.libraries.entry(m.to_string()).or_insert_with(||{
            match File::open(format!("machines/{}",machine_name)) {
              Ok(_) => {
                match &outgoing {
                  Some(sender) => {sender.send(ClientMessage::String(format!("{} {} v{}", "[Loading]".bright_cyan(), m, ver)));}
                  None => (),
                }
                let message = format!("Can't load library {:?}", machine_name);
                unsafe{Library::new(format!("machines/{}",machine_name)).expect(&message)}
              }
              _ => download_machine(&machine_name, m, path, ver, outgoing.clone()).unwrap()
            }
          });       
          let native_rust = unsafe {
            // Replace slashes with underscores and then add a null terminator
            let mut s = format!("{}\0", fun_name.replace("-","__").replace("/","_"));
            let error_msg = format!("Symbol {} not found",s);
            let m = library.get::<extern "C" fn(arguments: &Vec<(u64, ValueIterator)>)>(s.as_bytes()).expect(&error_msg);
            m.into_raw()
          };
          *fun = Some(*native_rust);
          // Resolve any function needed errors
          let mut resolved_errors = vec![];
          for error in &self.mech.runtime.errors {
            match error.error_type {
              ErrorType::MissingFunction(missing_function_id) => {
                if missing_function_id == *fun_name_id {
                  let block = self.mech.runtime.blocks.get_mut(&error.block_id).unwrap();
                  block.errors.remove(&error);
                  block.state = BlockState::New;
                  if block.is_ready() {
                    self.mech.runtime.ready_blocks.insert(block.id);
                  }
                  resolved_errors.push(error.clone());
                }
              }
              _ => (),
            }
          }
          for error in resolved_errors {
            self.mech.runtime.errors.remove(&error);
          }
          
        },
        _ => (),
      }
    }

    // Dedupe needed ids
    let registers = self.mech.runtime.needed_registers.difference(&self.mech.runtime.defined_registers);
    let mut needed_tables = HashSet::new();
    for register in registers {
      let database = self.mech.runtime.database.borrow();
      let needed_table_id = register.table_id.unwrap();
      needed_tables.insert(needed_table_id.clone());
    }
    
    let mut machine_init_code = vec![];
    for needed_table_id in needed_tables.iter() {
      let database = self.mech.runtime.database.borrow();
      let needed_table_name = database.store.strings.get(&needed_table_id).unwrap().clone();
      let m: Vec<_> = needed_table_name.split('/').collect();
      let needed_machine_id = hash_string(&m[0]);
      match self.loaded_machines.contains(&needed_machine_id) {
        false => {
          self.loaded_machines.insert(needed_machine_id);
          #[cfg(unix)]
          let machine_name = format!("libmech_{}.so", m[0]);
          #[cfg(windows)]
          let machine_name = format!("mech_{}.dll", m[0]);
          match self.machine_repository.get(m[0]) {
            Some((ver, path)) => {
              let library = self.libraries.entry(m[0].to_string()).or_insert_with(||{
                match File::open(format!("machines/{}",machine_name)) {
                  Ok(_) => {
                    match &outgoing {
                      Some(sender) => {sender.send(ClientMessage::String(format!("{} {} v{}", "[Loading]".bright_cyan(), m[0], ver)));}
                      None => (),
                    }
                    let message = format!("Can't load library {:?}", machine_name);
                    unsafe{Library::new(format!("machines/{}",machine_name)).expect(&message)}
                  }
                  _ => download_machine(&machine_name, m[0], path, ver, outgoing.clone()).unwrap()
                }
              });          
              // Replace slashes with underscores and then add a null terminator
              let mut s = format!("{}\0", needed_table_name.replace("-","__").replace("/","_"));
              let error_msg = format!("Symbol {} not found",s);
              let mut registrar = Registrar::new();
              unsafe{
                match library.get::<*mut MachineDeclaration>(s.as_bytes()) {
                  Ok(good) => {
                    let declaration = good.read();
                    let init_code = (declaration.register)(&mut registrar, self.outgoing.clone());
                    machine_init_code.push(init_code);
                  }
                  Err(_) => {
                    println!("Couldn't find the specified machine: {}", needed_table_name);
                  }
                }
              }        
              self.machines.extend(registrar.machines);
            },
            _ => (),
          }
        }
        _ => (),
      }
    }
    for program in &machine_init_code {
      self.compile_program(program.to_string());
      self.trigger_machines();
    }
    self.mech.step();
    self.trigger_machines();

    /*
    // Do it for the the other core
    for core in self.cores.values_mut() {
      for (fun_name, fun) in core.runtime.functions.iter_mut() {
        let m: Vec<_> = fun_name.split('/').collect();
        #[cfg(unix)]
        let machine_name = format!("libmech_{}.so", m[0]);
        #[cfg(windows)]
        let machine_name = format!("mech_{}.dll", m[0]);
        match (&fun, self.machine_repository.get(m[0])) {
          (None, Some((ver, path))) => {
            let library = self.libraries.entry(m[0].to_string()).or_insert_with(||{
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
              let m = library.get::<extern "C" fn(Vec<(String, Rc<RefCell<Table>>)>, Rc<RefCell<Table>>)>(s.as_bytes()).expect(&error_msg);
              m.into_raw()
            };
            *fun = Some(*native_rust);
          },
          _ => (),
        }
      }
    }*/
    
    Ok(())
  }

  pub fn clear(&mut self) {
    //self.mech.clear();
  }

}