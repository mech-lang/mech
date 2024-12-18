#![allow(warnings)]
// # Mech

// ## Prelude

pub extern crate mech_core as core;
pub extern crate mech_syntax as syntax;
//pub extern crate mech_program as program;
//pub extern crate mech_utilities as utilities;

//mod repl;

pub use mech_core::*;
use mech_core::nodes::Program;
use mech_interpreter::Interpreter;
//pub use mech_syntax::compiler::*;
//pub use mech_program::*;
//pub use mech_utilities::*;
//pub use self::repl::*;

extern crate colored;
use colored::*;

extern crate bincode;
use std::io::{Write, BufReader, BufWriter, stdout};
use std::fs::{OpenOptions, File, canonicalize, create_dir};

use tabled::{
  builder::Builder,
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
#[macro_use]
extern crate lazy_static;

lazy_static! {
  static ref CORE_MAP: Mutex<HashMap<SocketAddr, (String, SystemTime)>> = Mutex::new(HashMap::new());
}

pub fn pretty_print_tree(tree: &Program) -> String {
  let tree_hash = hash_str(&format!("{:#?}", tree));
  let formatted_tree = format_parse_tree(tree);
  let mut builder = Builder::default();
  builder.push_record(vec![format!("Hash: {}", tree_hash)]);
  builder.push_record(vec![format!("{}", formatted_tree)]);
  let mut table = builder.build();
  table.with(Style::modern())
       .with(Panel::header("🌳 Syntax Tree"));
  format!("{table}")
}

pub fn whos(intrp: &Interpreter) -> String {
  let mut builder = Builder::default();
  builder.push_record(vec!["Name","Size","Bytes","Kind"]);
  let symbol_table = intrp.symbols.borrow();
  for (id,name) in &symbol_table.dictionary {
    let value = symbol_table.get(*id).unwrap();
    let value_brrw = value.borrow();
    builder.push_record(vec![
      name.clone(),
      format!("{:?}",value_brrw.shape()),
      format!("{:?}",value_brrw.size_of()),
      format!("{:?}",value_brrw.kind())
    ]);
  }

  let mut table = builder.build();
  table.with(Style::modern());
  format!("{table}")
}

pub fn pretty_print_plan(intrp: &Interpreter) -> String {
  let mut builder = Builder::default();

  let mut row = vec![];
  let plan = intrp.plan.borrow();
  if plan.is_empty() {
    builder.push_record(vec!["".to_string()]);
  } else {
    for (ix, fxn) in plan.iter().enumerate() {
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
  table.with(Style::modern())
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

//extern crate nom;

/*
pub fn read_mech_files(mech_paths: &Vec<String>) -> Result<Vec<(String,MechCode)>, MechError> {

  let mut code: Vec<(String,MechCode)> = Vec::new();

  let read_file_to_code = |path: &Path| -> Result<Vec<(String,MechCode)>, MechError> {
    let mut code: Vec<(String,MechCode)> = Vec::new();
    match (path.to_str(), path.extension())  {
      (Some(name), Some(extension)) => {
        match extension.to_str() {
          Some("blx") => {
            match File::open(name) {
              Ok(file) => {
                println!("{} {}", "[Loading]".truecolor(153,221,85), name);
                let mut reader = BufReader::new(file);
                let mech_code: Result<MechCode, bincode::Error> = bincode::deserialize_from(&mut reader);
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
          }
          Some("mec") | Some("🤖") => {
            match File::open(name) {
              Ok(mut file) => {
                println!("{} {}", "[Loading]".truecolor(153,221,85), name);
                let mut buffer = String::new();
                file.read_to_string(&mut buffer);
                code.push((name.to_string(),MechCode::String(buffer)));
              }
              Err(err) => {
                return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1249, kind: MechErrorKind::None});
              },
            };
          }
          _ => (), // Do nothing if the extension is not recognized
        }
      },
      err => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1250, kind: MechErrorKind::GenericError(format!("{:?}", err))});},
    }
    Ok(code)
  };

  for path_str in mech_paths {
    let path = Path::new(path_str);
    // Compile a .mec file on the web
    if path.to_str().unwrap().starts_with("https") {
      println!("{} {}", "[Downloading]".truecolor(153,221,85), path.display());
      match reqwest::blocking::get(path.to_str().unwrap()) {
        Ok(response) => {
          match response.text() {
            Ok(text) => code.push((path.to_str().unwrap().to_owned(),MechCode::String(text))),
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
            let path = entry.path();
            let mut new_code = read_file_to_code(&path)?;
            code.append(&mut new_code);
          }
        }
      } else if path.is_file() {
        // Compile a single file
        let mut new_code = read_file_to_code(&path)?;
        code.append(&mut new_code);
      } else {
        return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1243, kind: MechErrorKind::FileNotFound(path.to_str().unwrap().to_string())});
      }
    };
  }
  Ok(code)
}*/

/*
pub fn compile_code(code: Vec<(String,MechCode)>) -> Result<Vec<Vec<MiniBlock>>,MechError> {
  print!("{}", "[Compiling] ".truecolor(153,221,85));
  stdout().flush();
  let mut sections = vec![];
  let now = Instant::now();
  for (_,c) in code {
    match c {
      MechCode::MiniCores(cores) => {
        todo!()
      }
      MechCode::String(c) => {
        let mut compiler = Compiler::new();
        let compiled = compiler.compile_str(&c)?;
        let mut mb = minify_blocks(&compiled);
        sections.append(&mut mb);
      },
      MechCode::MiniBlocks(mut mb) => {
        sections.append(&mut mb);
      },
    }
  }
  let elapsed_time = now.elapsed();
  let mut blocks_total = 0;
  for s in &sections {
    blocks_total += s.len();
  }
  println!("Compiled {} blocks in {}ms.", blocks_total, elapsed_time.as_micros() as f64 / 1000.0);
  Ok(sections)
}

pub fn minify_blocks(sections: &Vec<Vec<SectionElement>>) -> Vec<Vec<MiniBlock>> {
  let mut mb_sections = vec![];
  for section in sections {
    let mut miniblocks = Vec::new();
    for element in section {

      match element {
        SectionElement::Block(block) => {
          let miniblock = MiniBlock::minify_block(&block);
          miniblocks.push(miniblock);
        }
        _ => (),
      }
    }
    mb_sections.push(miniblocks);
  }
  mb_sections
}*/