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
//use websocket::sync::Server;
use std::net::{SocketAddr, UdpSocket, TcpListener, TcpStream};
use std::collections::HashMap;
use crossbeam_channel::Sender;
use std::{fs,env};
#[macro_use]
extern crate lazy_static;

#[cfg(feature = "wasm")]
use web_sys::{Crypto, Window, console};
use rand::rngs::OsRng;
use rand::RngCore;
use notify::{recommended_watcher, Event, RecursiveMode, Result as NResult, Watcher};
use std::sync::mpsc;
use std::sync::Arc;

lazy_static! {
  static ref CORE_MAP: Mutex<HashMap<SocketAddr, (String, SystemTime)>> = Mutex::new(HashMap::new());
}

mod repl;
mod serve;
mod run;

pub use self::repl::*;
pub use self::serve::*;
pub use self::run::*;

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
    ":help".to_string(),
    ":h".to_string(),
    "".to_string(),
    "Display this help message".to_string()
  ]);
  builder.push_record(vec![
    ":quit".to_string(),
    ":q".to_string(),
    "".to_string(),
    "Quit the REPL".to_string()
  ]);
  builder.push_record(vec![
    ":symbols".to_string(),
    ":s".to_string(),
    "[search pattern]".to_string(),
    "Search symbols".to_string()
  ]);
  builder.push_record(vec![
    ":plan".to_string(),
    ":p".to_string(),
    "".to_string(),
    "Display the plan".to_string()
  ]);
  builder.push_record(vec![
    ":whos".to_string(),
    ":w".to_string(),
    "[search pattern]".to_string(),
    "Search symbol directory".to_string()
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
    ":cd".to_string(),
    "".to_string(),
    "[target path]".to_string(),
    "Change directory".to_string()
  ]);
  builder.push_record(vec![
    ":step".to_string(),
    "".to_string(),
    "[step count]".to_string(),
    "Iterate plan".to_string()
  ]);
  let mut table = builder.build();
  table.with(mech_table_style())
       .with(Panel::header(format!("{}","🧭 Help".truecolor(0xdf,0xb9,0x9f))));
  format!("\n{table}\n")
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

pub fn pretty_print_tree(tree: &Program) -> String {
  let tree_hash = hash_str(&format!("{:#?}", tree));
  let formatted_tree = format_parse_tree(tree);
  let mut builder = Builder::default();
  builder.push_record(vec![format!("Hash: {}", tree_hash)]);
  builder.push_record(vec![format!("{}", formatted_tree)]);
  let mut table = builder.build();
  table.with(Style::modern_rounded())
       .with(Panel::header("🌳 Syntax Tree"));
  format!("{table}")
}

pub fn whos(intrp: &Interpreter) -> String {
  let mut builder = Builder::default();
  builder.push_record(vec!["Name","Size","Bytes","Kind"]);
  let dictionary = intrp.dictionary();
  for (id,name) in dictionary.borrow().iter() {
    let value = intrp.get_symbol(*id).unwrap();
    let value_brrw = value.borrow();
    builder.push_record(vec![
      name.clone(),
      format!("{:?}",value_brrw.shape()),
      format!("{:?}",value_brrw.size_of()),
      format!("{:?}",value_brrw.kind()),
    ]);
  }

  let mut table = builder.build();
  table.with(mech_table_style())   
        .with(Panel::header(format!("{}","🔍 Whos".truecolor(0xdf,0xb9,0x9f))));
  format!("\n{table}\n")
}

pub fn pretty_print_symbols(intrp: &Interpreter) -> String {
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

pub fn pretty_print_plan(intrp: &Interpreter) -> String {
  let mut builder = Builder::default();

  let mut row = vec![];
  let plan = intrp.plan();
  let plan_brrw = plan.borrow();
  if plan_brrw.is_empty() {
    builder.push_record(vec!["".to_string()]);
  } else {
    for (ix, fxn) in plan_brrw.iter().enumerate() {
      let plan_str = format!("{}. {}\n", ix + 1, fxn.to_string());
      row.push(plan_str.clone());
      if row.len() == 4 {
        builder.push_record(row.clone());
        row.clear();
      }
    }
  }
  if row.is_empty() == false {
    builder.push_record(row.clone());
  }
  let mut table = builder.build();
  table.with(Style::modern_rounded())
       .with(Panel::header("📋 Plan"));
  format!("{table}")
}

pub fn format_parse_tree(program: &Program) -> String {
  let json_string = serde_json::to_string_pretty(&program).unwrap();

  let depth = |line: &str|->usize{line.chars().take_while(|&c| c == ' ').count()};
  let mut result = String::new();
  let lines: Vec<&str> = json_string.lines().collect();
  result.push_str("Program\n");
  for (index, line) in lines.iter().enumerate() {
    let trm = line.trim();
    if trm == "}" || 
       trm == "},"|| 
       trm == "{" || 
       trm == "[" || 
       trm == "],"|| 
       trm == "]" {
      continue;
    }

    // Count leading spaces to determine depth
    let d = depth(line);
    // Construct the tree-like prefix
    let mut prefix = String::new();
    for _ in 0..d {
      prefix.push_str(" ");
    }
    if index == lines.len() {
      prefix.push_str("└ ");
    } else {
      if depth(lines[index + 1]) != d {
        prefix.push_str("└ ");
      } else {
        prefix.push_str("├ ");
      }
    }
    let trm = line.trim();
    let trm = trm.trim_end_matches('{')
                  .trim_start_matches('"')
                  .trim_end_matches(':')
                  .trim_end_matches('"')
                  .trim_end_matches('[');
    prefix.push_str(trm);

    // Append formatted line to result
    result.push_str(&prefix);
    result.push('\n');
    result = result.replace("\":", "");
  }
  let mut indexed_str = IndexedString::new(&result);
  'rows: for i in 0..indexed_str.rows {
    let rowz = &indexed_str.index_map[i];
    for j in 0..rowz.len() {
      let c = match indexed_str.get(i,j) {
        Some(c) => c,
        None => continue,
      };
      if c == '└' {
        for k in i+1..indexed_str.rows {
          match indexed_str.get(k,j) {
            Some(c2) => {
              if c2 == '└' {
                indexed_str.set(i,j,'├');
                for l in i+1..k {
                  match indexed_str.get(l,j) {
                    Some(' ') => {indexed_str.set(l,j,'│');},
                    Some('└') => {indexed_str.set(l,j,'├');},
                    _ => (),
                  }
                }
              } else if c2 == ' ' {
                continue;
              } else {
                continue 'rows;
              }
            },
            None => continue,
          }

        }
      } else if c == ' ' || c == '│' {
        continue;
      } else {
        continue 'rows;
      }
    }
  }
  indexed_str.to_string()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IndexedString {
  pub data: Vec<char>,
  pub index_map: Vec<Vec<usize>>,
  pub rows: usize,
  pub cols: usize,
}

impl IndexedString {
  fn new(input: &str) -> Self {
      let mut data = Vec::new();
      let mut index_map = Vec::new();
      let mut current_row = 0;
      let mut current_col = 0;
      index_map.push(Vec::new());
      for c in input.chars() {
        data.push(c);
        if c == '\n' {
          index_map.push(Vec::new());
          current_row += 1;
          current_col = 0;
        } else {
          index_map[current_row].push(data.len() - 1);
          current_col += 1;
        }
      }
      let rows = index_map.len();
      let cols = if rows > 0 { index_map[0].len() } else { 0 };
      IndexedString {
          data,
          index_map,
          rows,
          cols,
      }
  }
  fn to_string(&self) -> String {
    self.data.iter().collect()
  }
  fn get(&self, row: usize, col: usize) -> Option<char> {
    if row < self.rows {
      let rowz = &self.index_map[row];
      if col < rowz.len() {
        let index = self.index_map[row][col];
        Some(self.data[index])
      } else {
        None
      }
    } else {
      None
    }
  }

  fn set(&mut self, row: usize, col: usize, new_char: char) -> Result<(), String> {
    if row < self.rows {
      let row_indices = &mut self.index_map[row];
      if col < row_indices.len() {
        let index = row_indices[col];
        self.data[index] = new_char;
        Ok(())
      } else {
        Err("Column index out of bounds".to_string())
      }
    } else {
      Err("Row index out of bounds".to_string())
    }
  }
}

pub struct MechFileSystem {
  sources: Arc<Mutex<HashMap<u64,MechSourceCode>>>,
  watchers: Vec<Box<dyn Watcher>>,
  threads: Vec<JoinHandle<()>>,
}

// Start by taking the paths, which are strings. The paths may be .mec or robot files, or they may be directories
// For each directory and file, we will create a watcher. The watcher will watch for changes to the files and directories
// When a change is detected, the file will be read and the source code will be updated in the sources hashmap

impl MechFileSystem {

  pub fn new() -> Self {
    MechFileSystem {
      sources: Arc::new(Mutex::new(HashMap::new())),
      watchers: Vec::new(),
      threads: Vec::new(),
    }
  }
  
  pub fn watch_source(&mut self, src: &'static str) -> MResult<()> {
    let src_path = match Path::new(src.clone()).canonicalize() {
      Ok(path) => path,
      Err(e) => {
        return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1246, kind: MechErrorKind::GenericError(format!("{:?}", e))});
      },
    };

    let (tx, rx) = mpsc::channel::<NResult<Event>>();
    match notify::recommended_watcher(tx) {
      Ok(mut watcher) => {
        match watcher.watch(&src_path, RecursiveMode::Recursive) {
          Ok(_) => {
            println!("{} Watching: {}", "[Watch]".truecolor(153,221,85), src_path.display());
            let srcs = self.sources.clone();
            let thread = thread::spawn(move || {
              for res in rx {
                match res {
                  Ok(event) => {
                    match event.kind {
                      notify::EventKind::Modify(knd) => {
                        for p in event.paths {
                          match p.canonicalize() {
                            Ok(event_path) => {
                              match srcs.lock() {
                                Ok(mut sources) => {
                                  let file_id = hash_str(&event_path.display().to_string());
                                  match sources.get_mut(&file_id) {
                                    Some(code) => {
                                      let new_source = read_mech_source_file(&event_path).unwrap();
                                      *code = new_source;
                                    }
                                    None => (),
                                  }
                                },
                                Err(e) => {
                                  println!("watch error: {:?}", e);
                                },
                              }
                            }
                            Err(e) => println!("watch error: {:?}", e),
                          }
                        }
                      }
                      notify::EventKind::Create(_) => todo!(),
                      notify::EventKind::Remove(_) => todo!(),
                      _ => todo!(),
                    }
                  }
                  Err(e) => println!("watch error: {:?}", e),
                }
              }
            });
            self.watchers.push(Box::new(watcher));
            self.threads.push(thread);
          },
          Err(err) => {
            println!("{} Error watching: {}", "[Watch]".truecolor(153,221,85), err);
          },
        }
      },
      Err(err) => {
        println!("{} Error creating watcher: {}", "[Watch]".truecolor(153,221,85), err);
      },
    }
    Ok(())
  }

  pub fn read_mech_files(&mut self, mech_paths: &Vec<String>) -> MResult<Vec<(String,MechSourceCode)>> {
    let mut code: Vec<(String,MechSourceCode)> = Vec::new();
  
    for path_str in mech_paths {
      let path = Path::new(path_str);
      // Compile a .mec file on the web
      if path_str.starts_with("https") || path_str.starts_with("http") {
        println!("{} {}", "[Downloading]".truecolor(153,221,85), path.display());
        match reqwest::blocking::get(path_str) {
          Ok(response) => {
            match response.text() {
              Ok(text) => {
                let src = MechSourceCode::String(text);

                code.push((path_str.to_owned(), src));
              },
              _ => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1241, kind: MechErrorKind::None});},
            }
          }
          _ => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1242, kind: MechErrorKind::None});},
        }
      } else {
        // Compile a directory of mech files
        if path.is_dir() {
          for entry in path.read_dir().expect("read_dir call failed") {
            if let Ok(entry) = entry {
              let path = entry.path().canonicalize().unwrap();
              let mut mech_src = read_mech_source_file(&path)?;
              match self.sources.lock() {
                Ok(mut sources) => {
                  sources.insert(hash_str(&path.display().to_string()), mech_src.clone());
                },
                Err(err) => {
                  return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1244, kind: MechErrorKind::GenericError(format!("{:?}", err))});
                },
              }
              // path buf to string
              code.push((path.display().to_string(),mech_src));
            }
          }
        } else if path.is_file() {
          let mut mech_src = read_mech_source_file(&path)?;
          match self.sources.lock() {
            Ok(mut sources) => {
              sources.insert(hash_str(&path.canonicalize().unwrap().display().to_string()), mech_src.clone());
            },
            Err(err) => {
              return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1244, kind: MechErrorKind::GenericError(format!("{:?}", err))});
            },
          }
          // path buf to string
          code.push((path.display().to_string(),mech_src));
        } else {
          return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1243, kind: MechErrorKind::FileNotFound(path.to_str().unwrap().to_string())});
        }
      };
    }
    Ok(code)
  }
  
}

pub fn read_mech_source_file(path: &Path) -> MResult<MechSourceCode> {
  match path.extension() {
    Some(extension) => {
      match extension.to_str() {
        /*Some("blx") => {
          match File::open(name) {
            Ok(file) => {
              println!("{} {}", "[Loading]".truecolor(153,221,85), name);
              let mut reader = BufReader::new(file);
              let mech_code: Result<MechSourceCode, bincode::Error> = bincode::deserialize_from(&mut reader);
              match mech_code {
                Ok(c) => {code.push((name.to_string(),c));},
                Err(err) => {
                  return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1247, kind: MechErrorKind::GenericError(format!("{:?}", err))});
                },
              }
            }
            Err(err) => {
              return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1248, kind: MechErrorKind::None});
            },
          };
        }*/
        Some("mec") | Some("🤖") => {
          match File::open(path) {
            Ok(mut file) => {
              println!("{} {}", "[Loading]".truecolor(153,221,85), path.display());
              let mut buffer = String::new();
              file.read_to_string(&mut buffer);
              Ok(MechSourceCode::String(buffer))
            }
            Err(err) => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1249, kind: MechErrorKind::None}),
          }
        }
        Some("csv") => {
          match File::open(path) {
            Ok(mut file) => {
              println!("{} {}", "[Loading]".truecolor(153,221,85), path.display());
              let mut buffer = String::new();
              let mut rdr = csv::Reader::from_reader(file);
              for result in rdr.records() {
                println!("{:?}", result);
              }
              todo!();
            }
            Err(err) => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1249, kind: MechErrorKind::None}),
          }
        }
        _ => todo!(), // Do nothing if the extension is not recognized
      }
    },
    err => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1250, kind: MechErrorKind::GenericError(format!("{:?}", err))}),
  }
}