// # Program

// # Prelude
extern crate bincode;
extern crate libloading;
extern crate colored;
use colored::*;

use std::thread::{self, JoinHandle};
use std::collections::hash_map::Entry;
use std::mem;
use std::fs::{OpenOptions, File, canonicalize, create_dir};
use std::io::{Write, BufReader, BufWriter, Read};
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

use mech_core::*;
use mech_syntax::compiler::Compiler;
use mech_utilities::*;
use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;
use hashbrown::{HashSet, HashMap};

use super::download_machine;
use super::persister::Persister;
use super::runloop::ClientMessage;

use libloading::Library;
use std::io::copy;
use std::io;
use std::net::{SocketAddr, UdpSocket};

use time;

lazy_static! {
  static ref MECH_CODE: u64 = hash_str("mech/code");
  static ref MECH_REGISTRY: u64 = hash_str("mech/registry");
  static ref NAME: u64 = hash_str("name");
  static ref VERSION: u64 = hash_str("version");
  static ref URL: u64 = hash_str("url");
}


struct Machines {
  machines: HashMap<u64, Box<dyn Machine>>,
}

impl Machines {
  fn new() -> Machines {
    Machines {
      machines: HashMap::default(),
    }
  }
}

impl MachineRegistrar for Machines {
  fn register_machine(&mut self, machine: Box<dyn Machine>) {
    self.machines.insert(machine.id(), machine);
  }
}

struct MechFunctions {
  mech_functions: HashMap<u64, Box<dyn MechFunctionCompiler>>,
}

impl MechFunctions {
  fn new() -> MechFunctions {
    MechFunctions {
      mech_functions: HashMap::default(),
    }
  }
}

impl MechFunctionRegistrar for MechFunctions {
  fn register_mech_function(&mut self, function_id: u64, mech_function_compiler: Box<dyn MechFunctionCompiler>) {
    self.mech_functions.insert(function_id, mech_function_compiler);
  }
}

// ## Program

pub struct Program {
  pub name: String,
  pub mech: Core,
  pub cores: HashMap<u64,Core>,
  pub remote_cores: HashMap<u64,MechSocket>,
  pub input_map: HashMap<(TableId,TableIndex,TableIndex),HashSet<u64>>,
  pub libraries: HashMap<String, Library>,
  pub machines: HashMap<u64, Box<dyn Machine>>,
  pub mech_functions: HashMap<u64, Box<dyn MechFunctionCompiler>>,
  pub machine_repository: HashMap<String, (String, String)>,  // (name, (version, url))
  capacity: usize,
  pub incoming: Receiver<RunLoopMessage>,
  pub outgoing: Sender<RunLoopMessage>,
  pub errors: HashSet<MechErrorKind>,
  programs: usize,
  loaded_machines: HashSet<u64>,
  pub listeners: HashMap<(TableId,TableIndex,TableIndex),HashSet<u64>>,
}

impl Program {
  pub fn new(name:&str, capacity: usize, recursion_limit: u64, outgoing: Sender<RunLoopMessage>, incoming: Receiver<RunLoopMessage>) -> Program {
    let mut mech = Core::new();
    //`   `mech.load_standard_library();
    let mech_code = hash_str("mech/code");
    //let txn = Transaction{changes: vec![Change::NewTable{table_id: mech_code, rows: 1, columns: 1}]};
    //mech.process_transaction(&txn);
    Program { 
      name: name.to_owned(), 
      capacity,
      machine_repository: HashMap::new(), 
      mech,
      remote_cores: HashMap::new(),
      cores: HashMap::new(),
      libraries: HashMap::new(),
      machines: HashMap::new(),
      mech_functions: HashMap::new(),
      loaded_machines: HashSet::new(),
      input_map: HashMap::new(),
      incoming,
      outgoing,
      errors: HashSet::new(),
      programs: 0,
      listeners: HashMap::new(),
    }
  }

  pub fn trigger_machines(&mut self) {
    /*
    let database = self.mech.runtime.database.borrow();
    for register in &self.mech.runtime.aggregate_changed_this_round {
      match self.machines.get_mut(&register.hash()) {
        // Invoke the machine!
        Some(mut machine) => {
          let table = database.tables.get(&register.table_id.unwrap()).unwrap().borrow();
          machine.on_change(&table);
        },
        _ => (), // TODO Warn user that the machine is not loaded!
      }
    }*/
  }

  pub fn compile_program(&mut self, input: String) {
    let mut compiler = Compiler::new();
    let programs = compiler.compile_str(&input.clone());
    for p in programs {
      self.mech.load_blocks(p);
    }
    //self.errors.append(&mut self.mech.runtime.errors.clone());
    let mech_code = *MECH_CODE;
    self.programs += 1;
    let txn = vec![Change::Set((mech_code, vec![(TableIndex::Index(self.programs),TableIndex::Index(1),Value::from_str(&input.clone()))]))];
    self.outgoing.send(RunLoopMessage::Transaction(txn));
  }

  pub fn compile_fragment(&mut self, input: String) {
    /*
    let mut compiler = Compiler::new();
    let programs = compiler.compile_string(input.clone());
    for p in programs {
      for mut block in p.blocks {
        block.id = (self.mech.runtime.blocks.len() + 1) as u64;
        self.mech.runtime.ready_blocks.insert(block.id);
        self.mech.register_blocks(vec![block]);
      }
    }
    //self.errors.append(&mut self.mech.runtime.errors.clone());
    let mech_code = *MECH_CODE;
    self.programs += 1;
    //let txn = Transaction::from_change(Change::Set{table: mech_code, row: TableIndex::Index(self.programs), column: TableIndex::Index(1), value: Value::from_str(&input.clone())});
    //self.outgoing.send(RunLoopMessage::Transaction(txn));
    */
  }

  pub fn download_dependencies(&mut self, outgoing: Option<crossbeam_channel::Sender<ClientMessage>>) -> Result<Vec<MechErrorKind>,MechError> {
    // Create the machines directory. If it's already there this does nothing.    
    create_dir("machines");
    // If the machine repository is not populated, we need to fill it by loading the registry
    if self.machine_repository.len() == 0 {
      let mut registry_file = match std::fs::File::open("machines/registry.mec") {
        Ok(mut file) => {
          // Loading machine_repository index
          match &outgoing {
            Some(sender) => {sender.send(ClientMessage::String(format!("{} Machine registry.", "[Loading]".bright_cyan())));}
            None => {return Err(MechError{id: 1234, kind: MechErrorKind::None});},
          }
          let mut contents = String::new();
          match file.read_to_string(&mut contents) {
            Err(_) => {return Err(MechError{id: 1234, kind: MechErrorKind::None});},
            _ => (),
          }
          contents
        }
        Err(_) => {
          // Download machine_repository index
          match &outgoing {
            Some(sender) => {sender.send(ClientMessage::String(format!("{} Updating machine registry.", "[Downloading]".bright_cyan())));}
            None => {return Err(MechError{id: 1234, kind: MechErrorKind::None});},
          }
          // Download registry
          let registry_url = "https://gitlab.com/mech-lang/machines/mech/-/raw/main/src/registry.mec";
          let mut response_text = match reqwest::get(registry_url) {
            Ok(mut response) => {
              match response.text() {
                Ok(text) => text,
                Err(_) => {return Err(MechError{id: 1234, kind: MechErrorKind::None});},
              }
            }
            Err(_) => {return Err(MechError{id: 1234, kind: MechErrorKind::None});},
          };
          // Save registry
          let mut dest = match File::create("machines/registry.mec") {
            Ok(dest) => dest,
            Err(_) => {return Err(MechError{id: 1234, kind: MechErrorKind::None});},
          };
          match dest.write_all(response_text.as_bytes()) {
            Ok(dest) => dest,
            Err(_) => {return Err(MechError{id: 1234, kind: MechErrorKind::None});},            
          }
          response_text
        }
      };
      
      // Compile machine registry
      let mut registry_compiler = Compiler::new();
      let blocks = registry_compiler.compile_str(&registry_file)?;
      let mut registry_core = Core::new();
      registry_core.load_blocks(blocks);

      // Convert the machine listing into a hash map
      let registry_table = registry_core.get_table("mech/registry")?;
      let registry_table_brrw = registry_table.borrow();
      for row in 0..registry_table_brrw.rows {
        let row_index = TableIndex::Index(row+1);
        let name = registry_table_brrw.get_by_index(row_index, TableIndex::Alias(*NAME))?.as_string().unwrap().to_string();
        let version = registry_table_brrw.get_by_index(row_index, TableIndex::Alias(*VERSION))?.as_string().unwrap().to_string();
        let url = registry_table_brrw.get_by_index(row_index, TableIndex::Alias(*URL))?.as_string().unwrap().to_string();
        self.machine_repository.insert(name, (version, url));
      }
    }
    // Resolve missing function errors
    let mut resolved_errors = vec![];
    {
      for (error,eblocks) in &self.mech.errors {
        match error {
          MechErrorKind::MissingFunction(fxn_id) => {
            let fun_name = self.mech.dictionary.borrow().get(&fxn_id).unwrap().to_string();
            let m: Vec<_> = fun_name.split('/').collect();
            let m = m[0];
            let underscore_name = m.replace("-","_");
            #[cfg(target_os = "macos")]
            let machine_name = format!("libmech_{}.dylib", underscore_name);
            #[cfg(target_os = "linux")]
            let machine_name = format!("libmech_{}.so", underscore_name);
            #[cfg(target_os = "windows")]
            let machine_name = format!("mech_{}.dll", underscore_name);
            match self.machine_repository.get(&m.to_string()) {
              Some((ver, path)) => {
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
                // Replace slashes with underscores and then add a null terminator
                let mut s = format!("{}\0", fun_name.replace("-","__").replace("/","_"));
                let error_msg = format!("Symbol {} not found",s);
                let mut registrar = MechFunctions::new();
                unsafe{
                  match library.get::<*mut MechFunctionDeclaration>(s.as_bytes()) {
                    Ok(good) => {
                      let declaration = good.read();
                      (declaration.register)(&mut registrar);
                    }
                    Err(_) => {
                      println!("Couldn't find the specified machine: {}", fun_name);
                    }
                  }
                }     
                self.mech.functions.borrow_mut().extend(registrar.mech_functions);
                resolved_errors.push(error.clone());
              }
              _ => (),
            }
          }
          _ => (), // Other error, do nothing
        }
      }
    }
    
    // Dedupe needed ids
    let needed_registers = self.mech.needed_registers();
    let mut needed_tables = HashSet::new();
    for (needed_table_id,_,_) in needed_registers {
      needed_tables.insert(needed_table_id.clone());
    }
    
    let mut machine_init_code = vec![];
    for needed_table_id in needed_tables.iter() {
      let dictionary = self.mech.dictionary.borrow();
      let needed_table_name = dictionary.get(&needed_table_id.unwrap()).unwrap().to_string();
      let m: Vec<_> = needed_table_name.split('/').collect();
      let needed_machine_id = hash_str(&m[0]);
      match self.loaded_machines.contains(&needed_machine_id) {
        false => {
          self.loaded_machines.insert(needed_machine_id);
          #[cfg(target_os = "macos")]
          let machine_name = format!("libmech_{}.dylib", m[0]);
          #[cfg(target_os = "linux")]
          let machine_name = format!("libmech_{}.so", m[0]);
          #[cfg(target_os = "windows")]
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
              let mut registrar = Machines::new();
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
    for mec in &machine_init_code {
      self.compile_program(mec.to_string());
      self.trigger_machines();
    }
    //self.mech.step();
    //self.trigger_machines();

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
    
    Ok(resolved_errors)
  }

  /*pub fn clear(&mut self) {
    self.mech.clear();
  }*/

}