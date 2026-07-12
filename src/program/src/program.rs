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
use std::time::{Instant};

use mech_core::*;
use mech_syntax::compiler::Compiler;
use mech_utilities::*;
use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;
use hashbrown::{HashSet, HashMap};
use indexmap::IndexSet;

use super::download_machine;
use super::persister::Persister;
use super::runloop::ClientMessage;

use libloading::Library;
use std::io::copy;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::fmt;

use time;

lazy_static! {
  static ref MECH_CODE: u64 = hash_str("mech/code");
  static ref MECH_REGISTRY: u64 = hash_str("mech/registry");
  static ref NAME: u64 = hash_str("name");
  static ref VERSION: u64 = hash_str("version");
  static ref URL: u64 = hash_str("url");
}

// ## Program

pub struct Program {
  pub id: u64,
  pub name: String,
  pub mech: Core,
  pub cores: HashMap<u64,Core>,
  pub remote_cores: HashMap<u64,MechSocket>,
  pub input_map: HashMap<(TableId,RegisterIndex,RegisterIndex),HashSet<u64>>,
  pub libraries: HashMap<String, Option<Library>>,
  pub machines: HashMap<u64, Box<dyn Machine>>,
  pub mech_functions: HashMap<u64, Box<dyn MechFunctionCompiler>>,
  pub machine_repository: HashMap<String, (String, String)>,  // (name, (version, url))
  capacity: usize,
  pub incoming: Receiver<RunLoopMessage>,
  pub outgoing: Sender<RunLoopMessage>,
  pub errors: HashSet<MechErrorKind>,
  programs: usize,
  loaded_machines: HashSet<u64>,
  pub listeners: HashMap<(TableId,RegisterIndex,RegisterIndex),HashSet<u64>>,
  pub trigger_to_listener: HashMap<(TableId,RegisterIndex,RegisterIndex),((TableId, RegisterIndex, RegisterIndex),HashSet<u64>)>,
  pub registry: String,
  pub capability_token: CapabilityToken,
}

impl Program {
  pub fn new(name:&str, capacity: usize, recursion_limit: u64, outgoing: Sender<RunLoopMessage>, incoming: Receiver<RunLoopMessage>, registry: String, default_caps: Option<HashSet<Capability>>) -> Program {

    let program_id = generate_uuid();

    let mut capability_token = CapabilityToken::new(name.into(),default_caps.unwrap(),program_id,None);
    let keypair = generate_keypair();
    capability_token.sign(&keypair);


    let mut mech = Core::new();
    Program { 
      id: program_id,
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
      trigger_to_listener: HashMap::new(),
      registry,
      capability_token,
    }
  }

  pub fn trigger_machine(&mut self, register: &(TableId,RegisterIndex,RegisterIndex)) -> Result<(),MechError> {
    let (table_id,_,_) = register;
    match self.machines.get_mut(table_id.unwrap()) {
      Some(mut machine) => {
        let table_ref = self.mech.get_table_by_id(*table_id.unwrap())?;
        let table_ref_brrw = table_ref.borrow();
        machine.on_change(&table_ref_brrw);
      },
      _ => (), // Warn user that the machine is not loaded? Or is it okay to just try?
    }
    Ok(())
  }

  pub fn compile_program(&mut self, input: String) -> Result<Vec<((Vec<BlockId>,Vec<u64>,Vec<MechError>))>,MechError> {
    let mut compiler = Compiler::new();
    let sections = compiler.compile_str(&input.clone())?;
    let result = self.mech.load_sections(sections);

    //self.errors.append(&mut self.mech.runtime.errors.clone());
    /*let mech_code = *MECH_CODE;
    self.programs += 1;
    let txn = vec![Change::Set((mech_code, vec![(TableIndex::Index(self.programs),TableIndex::Index(1),Value::from_str(&input.clone()))]))];
    self.outgoing.send(RunLoopMessage::Transaction(txn));*/
    Ok(result)    
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
            Some(sender) => {sender.send(ClientMessage::String(format!("{} Machine registry.", "[Loading]".truecolor(153,221,85))));}
            None => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1244, kind: MechErrorKind::None});},
          }
          let mut contents = String::new();
          match file.read_to_string(&mut contents) {
            Err(_) => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1445, kind: MechErrorKind::None});},
            _ => (),
          }
          contents
        }
        Err(_) => {
          // Download machine_repository index
          match &outgoing {
            Some(sender) => {sender.send(ClientMessage::String(format!("{} Updating machine registry from:\n{}", "[Downloading]".truecolor(153,221,85),self.registry)));}
            None => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1246, kind: MechErrorKind::None});},
          }
          // Download registry
          let registry_url = &self.registry;
          let mut response_text = match reqwest::get(registry_url) {
            Ok(mut response) => {
              match response.text() {
                Ok(text) => {
                  text
                },
                Err(_) => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1235, kind: MechErrorKind::None});},
              }
            }
            Err(_) => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1236, kind: MechErrorKind::None});},
          };
          // Save registry
          let mut dest = match File::create("machines/registry.mec") {
            Ok(dest) => dest,
            Err(_) => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1237, kind: MechErrorKind::None});},
          };
          match dest.write_all(response_text.as_bytes()) {
            Ok(dest) => dest,
            Err(_) => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1238, kind: MechErrorKind::None});},            
          }
          response_text
        }
      };
      
      // Compile machine registry
      let mut registry_compiler = Compiler::new();
      let sections = registry_compiler.compile_str(&registry_file)?;
      let mut registry_core = Core::new();
      registry_core.load_sections(sections);

      // Convert the machine listing into a hash map
      let registry_table = registry_core.get_table("mech/registry")?;
      let registry_table_brrw = registry_table.borrow();
      for row in 0..registry_table_brrw.rows {
        let row_index = TableIndex::Index(row+1);
        let name = registry_table_brrw.get_by_index(row_index.clone(), TableIndex::Alias(*NAME))?.as_string().unwrap().to_string();
        let version = registry_table_brrw.get_by_index(row_index.clone(), TableIndex::Alias(*VERSION))?.as_string().unwrap().to_string();
        let url = registry_table_brrw.get_by_index(row_index.clone(), TableIndex::Alias(*URL))?.as_string().unwrap().to_string();
        self.machine_repository.insert(name, (version, url));
      }
    }
    // Resolve missing function errors
    let mut resolved_errors = vec![];
    {
      let mut missing_functions: HashSet<u64> = HashSet::new();
      for (error,eblocks) in &self.mech.errors {
        match error {
          MechErrorKind::MissingFunction(fxn_id) => {
            missing_functions.insert(*fxn_id);
          }
          _ => (), // Other error, do nothing
        }
      }
      for fxn_id in &self.mech.required_functions {
        missing_functions.insert(*fxn_id);
      }

      for fxn_id in self.mech.functions.borrow().functions.keys() {
        missing_functions.remove(fxn_id);
      }

      // Iterate over the missing_functions
      for fxn_id in missing_functions {
        // Look up the function name using the Mech runtime's dictionary and the function ID
        let fun_name = self.mech.dictionary.borrow().get(&fxn_id).unwrap().to_string();
        let m: Vec<_> = fun_name.split('/').collect();
        let m = m[0];
        // Replace hyphens with underscores in the function name
        let underscore_name = m.replace("-","_");
        // Define the library file name based on the target operating system
        #[cfg(target_os = "macos")]
        let machine_name = format!("libmech_{}.dylib", underscore_name);
        #[cfg(target_os = "linux")]
        let machine_name = format!("libmech_{}.so", underscore_name);
        #[cfg(target_os = "windows")]
        let machine_name = format!("mech_{}.dll", underscore_name);
        // Check if the machine exists in the machine_repository
        match self.machine_repository.get(&m.to_string()) {
          Some((ver, path)) => {
            // Attempt to load the machine from an existing library file
            let library = self.libraries.entry(m.to_string()).or_insert_with(||{
              match File::open(format!("machines/{}",machine_name)) {
                Ok(_) => {
                  // Notify that the machine is loading
                  match &outgoing {
                    Some(sender) => {sender.send(ClientMessage::String(format!("{} {} v{}", "[Loading]".truecolor(153,221,85), m, ver)));}
                    None => (),
                  }
                  let message = format!("Can't load library {:?}", machine_name);
                  unsafe{Some(Library::new(format!("machines/{}",machine_name)).expect(&message))}
                }
                _ => Some(download_machine(&machine_name, m, path, ver, outgoing.clone()).unwrap())
              }
            });
            // Replace slashes with underscores in the function name and add a null terminator to the string
            let mut s = format!("{}\0", fun_name.replace("-","__").replace("/","_"));
            let error_msg = format!("Symbol {} not found",s);
            // Create a new MechFunctions struct to act as a registrar for the custom function
            let mut registrar = MechFunctions::new();
            // Use the library object to load the custom function
            unsafe {
              match library {
                Some(lib) => {
                  // Register the custom function with the registrar
                  match lib.get::<*mut MechFunctionDeclaration>(s.as_bytes()) {
                    Ok(good) => {
                      let declaration = good.read();
                      (declaration.register)(&mut registrar);
                    }
                    Err(_) => {
                      println!("Couldn't find the specified machine: {}", fun_name);
                    }
                  }
                }
                None => (),
              }
            }
            // Extend the runtime's functions collection with the new function and add a MechErrorKind::MissingFunction error to the resolved_errors list
            self.mech.functions.borrow_mut().extend(registrar.mech_functions);
            resolved_errors.push(MechErrorKind::MissingFunction(fxn_id));
          }
          _ => (),
        }
      }
    }
    
    // Dedupe needed ids
    let needed_registers = self.mech.needed_registers();
    let mut needed_tables = IndexSet::new();
    for (needed_table_id,_,_) in needed_registers {
      needed_tables.insert(needed_table_id.clone());
    }
    for (error,_) in &self.mech.errors {
      match error {
        MechErrorKind::MissingTable(table_id) => {
          needed_tables.insert(table_id.clone());
        }
        _ => (),
      }
    }

    let mut machine_init_code = vec![];

    // Iterate over the needed_tables, which represent the required custom functions.
    for needed_table_id in needed_tables.iter() {
      // Borrow the dictionary from the Mech runtime to look up the table name using the table ID.
      let dictionary = self.mech.dictionary.borrow();
      let needed_table_name = dictionary.get(needed_table_id.unwrap()).unwrap().to_string();
      // Split the table name into a vector of strings and compute the needed machine ID.
      let m: Vec<_> = needed_table_name.split('/').collect();
      let needed_machine_id = hash_str(&m[0]);
      // Check if the machine is already loaded by looking it up in the loaded_machines set.
      match self.loaded_machines.contains(&needed_machine_id) {
        false => {
          // If the machine is not loaded, insert the machine ID into loaded_machines.
          self.loaded_machines.insert(needed_machine_id);
          // Define the library file name based on the target operating system.
          #[cfg(target_os = "macos")]
          let machine_name = format!("libmech_{}.dylib", m[0]);
          #[cfg(target_os = "linux")]
          let machine_name = format!("libmech_{}.so", m[0]);
          #[cfg(target_os = "windows")]
          let machine_name = format!("mech_{}.dll", m[0]);
          // Check if the machine exists in the machine_repository. If it does, proceed with loading the machine.
          match self.machine_repository.get(m[0]) {
            Some((ver, path)) => {
              // Attempt to load the machine from an existing library file. If the file doesn't exist, download the machine using the download_machine function.
              let library = self.libraries.entry(m[0].to_string()).or_insert_with(||{
                match File::open(format!("machines/{}",machine_name)) {
                  Ok(_) => {
                    // If the library file exists, send a loading message to the outgoing client if it exists.
                    match &outgoing {
                      Some(sender) => {sender.send(ClientMessage::String(format!("{} {} v{}", "[Loading]".truecolor(153,221,85), m[0], ver)));}
                      None => (),
                    }
                    let message = format!("Can't load library {:?}", machine_name);
                    unsafe{Some(Library::new(format!("machines/{}",machine_name)).expect(&message))}
                  }
                  _ => {
                    // If the library file doesn't exist, download the machine and return it as an Option.
                    match download_machine(&machine_name, m[0], path, ver, outgoing.clone()) {
                      Ok(library) => Some(library),
                      Err(err) => None,
                    }
                  }
                }
              });
              // Replace slashes with underscores in the table name and add a null terminator to the string.
              let mut s = format!("{}\0", needed_table_name.replace("-","__").replace("/","_"));
              let error_msg = format!("Symbol {} not found",s);
              // Create a new Machines struct to act as a registrar for the custom function.
              let mut registrar = Machines::new();
              // Use the library object to load the custom function, register it with the registrar, and store the initialization code in machine_init_code.
              unsafe{
                match library {
                  Some(lib) => {
                    // Load the custom function using the library object.
                    match lib.get::<*mut MachineDeclaration>(s.as_bytes()) {
                      Ok(good) => {
                        let declaration = good.read();
                        // Register the custom function with the registrar.
                        match (declaration.register)(&mut registrar, self.outgoing.clone(), &self.capability_token) {
                          Ok(init_code) => machine_init_code.push(init_code),
                          Err(mech_error) => {return Err(mech_error)},
                        }
                      }
                      Err(_) => {
                        // If the custom function cannot be loaded, print an error message.
                        println!("Couldn't find the specified machine: {}", needed_table_name);
                      }
                    }                  
                  }
                  None => (),
                }
              }        
              // If the custom function is successfully loaded, extend the runtime's machines collection with the new machine.
              self.machines.extend(registrar.machines);
            },
            _ => (),
          }
        }
        _ => (),
      }
    }

    // Load init code and trigger machines
    let mut already_triggered = HashSet::new();
    for mic in &machine_init_code {
      let result = self.compile_program(mic.to_string())?;
      self.mech.schedule_blocks();
      for (new_block_ids,_,block_error) in result {
        for block_id in new_block_ids {
          let block = self.mech.blocks.get(&block_id);
          let output = self.mech.get_output_by_block_id(block_id)?;
          for register in output.iter() {
            if !already_triggered.contains(register) {
              self.trigger_machine(register);
            }
            already_triggered.insert(register.clone());
          }
        }
      }
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

impl fmt::Debug for Program {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut box_drawing = BoxPrinter::new();
    box_drawing.add_title("🤖","Program");
    box_drawing.add_title("  ","cores");
    box_drawing.add_line(format!("  1. (b {:?}, t {:?})", self.mech.blocks.len() , self.mech.database.borrow().tables.len()));
    for (ix, core) in self.cores.iter() {
      box_drawing.add_line(format!("  {:?}. (b {:?}, t {:?})", ix, core.blocks.len() , core.database.borrow().tables.len() ));
    }
    write!(f,"{:?}",box_drawing)?;
    Ok(())
  }
}