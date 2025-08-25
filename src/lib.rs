#![allow(warnings)]
// # Mech

// ## Prelude

pub extern crate mech_core as core;
pub extern crate mech_syntax as syntax;

pub use mech_core::*;
use mech_core::nodes::Program;
pub use mech_interpreter::Interpreter;

extern crate colored;
use colored::*;

extern crate bincode;
use std::io::{Write, BufReader, BufWriter, stdout};
use std::fs::{OpenOptions, File, canonicalize, create_dir};
use crossterm::{
  ExecutableCommand, QueueableCommand,
  terminal, cursor, style::Print,
};

use tabled::{
  builder::Builder,
  grid::config::HorizontalLine,
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};
use std::path::{Path, PathBuf};
use std::io;
use std::io::prelude::*;
use std::time::{Duration, Instant, SystemTime};
use std::thread::{self, JoinHandle};
use std::sync::Mutex;
use std::sync::RwLock;
//use websocket::sync::Server;
use std::net::{SocketAddr, UdpSocket, TcpListener, TcpStream};
use std::collections::HashMap;
use crossbeam_channel::Sender;
use crossbeam_channel::{unbounded, Receiver};
use std::{fs,env};

#[cfg(feature = "wasm")]
use web_sys::{Crypto, Window, console};
use rand::rngs::OsRng;
use rand::RngCore;
use notify::{recommended_watcher, Event, RecursiveMode, Result as NResult, Watcher};
use std::sync::mpsc;
use std::sync::Arc;
use std::collections::HashSet;

#[cfg(feature = "repl")]
mod repl;
#[cfg(feature = "serve")]
mod serve;
#[cfg(feature = "run")]
mod run;
#[cfg(feature = "mechfs")]
mod mechfs;

#[cfg(feature = "repl")]
pub use self::repl::*;
#[cfg(feature = "serve")]
pub use self::serve::*;
#[cfg(feature = "run")]
pub use self::run::*;
#[cfg(feature = "mechfs")]
pub use self::mechfs::*;

pub use mech_core::*;
pub use mech_syntax::*;

// Print a prompt 
// 4, 8, 15, 16, 23, 42
pub fn print_prompt() {
  stdout().flush();
  print!("{}", ">: ".truecolor(246,192,78));
  stdout().flush();
}

// Generate a new id for creating unique owner ids
#[cfg(not(feature = "wasm"))]
pub fn generate_uuid() -> u64 {
  OsRng.next_u64()
}

#[cfg(feature = "wasm")]
pub fn generate_uuid() -> u64 {
  let mut rng = WebCryptoRng{};
  rng.next_u64()
}

pub fn mech_table_style() -> Style<(),(),(),(),(),(),2,0> {
  Style::empty()
    .horizontals([
      (1, HorizontalLine::filled('-').into()),
      (2, HorizontalLine::filled('-').into()),
    ])
}

pub fn help() -> String {
  let mut builder = Builder::default();
  builder.push_record(vec!["Command","Short","Parameters","Description"]);
  builder.push_record(vec![
    ":cd".to_string(),
    "".to_string(),
    "[target path]".to_string(),
    "Change directory".to_string()
  ]);
  builder.push_record(vec![
    ":clc".to_string(),
    ":c".to_string(),
    "".to_string(),
    "Clear the screen".to_string()
  ]);
  builder.push_record(vec![
    ":clear".to_string(),
    "".to_string(),
    "[target variable]".to_string(),
    "Clear the interpreter state".to_string()
  ]);
  builder.push_record(vec![
    ":docs".to_string(),
    ":d".to_string(),
    "[doc name]".to_string(),
    "Search documentation for a given doc".to_string()
  ]);
  builder.push_record(vec![
    ":help".to_string(),
    ":h".to_string(),
    "".to_string(),
    "Display this help message".to_string()
  ]);
  builder.push_record(vec![
    ":load".to_string(),
    "".to_string(),
    "[file path]".to_string(),
    "Load a file".to_string()
  ]);
  builder.push_record(vec![
    ":ls".to_string(),
    "".to_string(),
    "[target path]".to_string(),
    "List directory contents".to_string()
  ]);
  builder.push_record(vec![
    ":plan".to_string(),
    ":p".to_string(),
    "".to_string(),
    "Display the plan".to_string()
    ]);
  builder.push_record(vec![
    ":quit".to_string(),
    ":q".to_string(),
    "".to_string(),
    "Quit the REPL".to_string()
  ]);
  builder.push_record(vec![
    ":step".to_string(),
    "".to_string(),
    "[step count]".to_string(),
    "Iterate plan".to_string()
  ]);
  builder.push_record(vec![
    ":symbols".to_string(),
    ":s".to_string(),
    "[search pattern]".to_string(),
    "Search symbols".to_string()
  ]);
  builder.push_record(vec![
    ":whos".to_string(),
    ":w".to_string(),
    "[search pattern]".to_string(),
    "Search symbol directory".to_string()
  ]);
  let mut table = builder.build();
  table.with(mech_table_style())
       .with(Panel::header(format!("{}","🧭 Help".truecolor(0xdf,0xb9,0x9f))));
  format!("\n{table}\n")
}

// Create a function to handle file writing
pub fn save_to_file(path: PathBuf, content: &str) -> MResult<()> {
  if let Some(parent) = path.parent() {
    if let Err(err) = fs::create_dir_all(parent) {
      return Err(MechError {file: file!().to_string(),tokens: vec![],msg: format!("Error writing to file: {:?}", err),id: line!(),kind: MechErrorKind::None});
    }
  }
  match fs::File::create(&path) {
    Ok(mut file) => {
      match file.write_all(content.as_bytes()) {
        Ok(_) => {
          println!("{} File saved as {}", "[Save]".truecolor(153,221,85), path.display());
          Ok(())
        }
        Err(err) => Err(MechError {file: file!().to_string(),tokens: vec![],msg: format!("Error writing to file: {:?}", err),id: line!(),kind: MechErrorKind::None}),
      }
    },
    Err(err) => Err(MechError {file: file!().to_string(),tokens: vec![],msg: format!("Error writing to file: {:?}", err),id: line!(),kind: MechErrorKind::None}),
  }
}

pub fn ls() -> String {
  let current_dir = env::current_dir().unwrap();
  let mut builder = Builder::default();
  builder.push_record(vec!["Mode","Last Write Time","Length","Name"]);
  for entry in fs::read_dir("./").unwrap() {
    let entry = entry.unwrap();
    let path = entry.path();
    let metadata = fs::metadata(&path).unwrap();
    let file_type = if metadata.is_dir() { "d----" } else { "-a---" };
    let last_write_time = metadata.modified().unwrap();
    let last_write_time: chrono::DateTime<chrono::Local> = last_write_time.into();
    let length = if metadata.is_file() { metadata.len().to_string() } else { "".to_string() };
    let name = format!("{}", path.file_name().unwrap().to_str().unwrap());
    builder.push_record(vec![file_type.to_string(), last_write_time.format("%m/%d/%Y %I:%M %p").to_string(), length, name.to_string()]);
  }
  let mut table = builder.build();
  table.with(mech_table_style())
       .with(Panel::header(format!("{}","📂 Directory Listing".truecolor(0xdf,0xb9,0x9f))));
  format!("\nDirectory: {}\n\n{table}\n",current_dir.display())
}

#[cfg(feature = "pretty_print")]
fn pretty_print_tree(tree: &Program) -> String {
  let tree_hash = hash_str(&format!("{:#?}", tree));
  let formatted_tree = tree.pretty_print();
  let mut builder = Builder::default();
  builder.push_record(vec![format!("Hash: {}", tree_hash)]);
  builder.push_record(vec![format!("{}", formatted_tree)]);
  let mut table = builder.build();
  table.with(Style::modern_rounded())
       .with(Panel::header("🌳 Syntax Tree"));
  format!("{table}")
}

#[cfg(all(feature = "pretty_print", feature = "variables"))]
pub fn whos(intrp: &Interpreter, names: Vec<String>) -> String {
  let mut builder = Builder::default();
  builder.push_record(vec!["Name", "Size", "Bytes", "Kind"]);

  let dictionary = intrp.dictionary();

  if names.is_empty() {
    // Print all symbols
    for (id, name) in dictionary.borrow().iter() {
      let value = intrp.state.borrow().get_symbol(*id).unwrap();
      let value_brrw = value.borrow();
      builder.push_record(vec![
        name.clone(),
        format!("{:?}", value_brrw.shape()),
        format!("{}", value_brrw.size_of()),
        format!("{}", value_brrw.kind()),
      ]);
    }
  } else {
    // Create a hash set for fast lookup
    let names_set: HashSet<_> = names.iter().collect();

    // Print only symbols in names
    for (id, name) in dictionary.borrow().iter() {
      if names_set.contains(name) {
        let value = intrp.state.borrow().get_symbol(*id).unwrap();
        let value_brrw = value.borrow();
        builder.push_record(vec![
          name.clone(),
          format!("{:?}", value_brrw.shape()),
          format!("{}", value_brrw.size_of()),
          format!("{}", value_brrw.kind()),
        ]);
      }
    }
  }
  let mut table = builder.build();
  table.with(mech_table_style())
      .with(Panel::header(format!("{}","🔍 Whos".truecolor(0xdf,0xb9,0x9f))));

  format!("\n{table}\n")
}


#[cfg(feature = "pretty_print")]            
fn pretty_print_symbols(intrp: &Interpreter) -> String {
  let mut builder = Builder::default();
  let symbol_table = intrp.pretty_print_symbols();
  builder.push_record(vec![
    format!("{}",symbol_table),
  ]);

  let mut table = builder.build();
  table.with(mech_table_style())   
        .with(Panel::header(format!("{}","🔣 Symbols".truecolor(0xdf,0xb9,0x9f))));
  format!("\n{table}\n")
}

pub fn clc() {
  let mut stdo = stdout();
  stdo.execute(terminal::Clear(terminal::ClearType::All));
  stdo.execute(cursor::MoveTo(0,0));
}