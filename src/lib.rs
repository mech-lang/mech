#![allow(warnings)]
// # Mech

// ## Prelude

pub extern crate mech_core as core;
pub extern crate mech_syntax as syntax;
//pub extern crate mech_program as program;
pub extern crate mech_utilities as utilities;

//mod repl;

pub use mech_core::*;
pub use mech_syntax::compiler::*;
//pub use mech_program::*;
pub use mech_utilities::*;
//pub use self::repl::*;

extern crate colored;
use colored::*;

extern crate bincode;
use std::io::{Write, BufReader, BufWriter, stdout};
use std::fs::{OpenOptions, File, canonicalize, create_dir};

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
                    return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1247, kind: MechErrorKind::GenericError(format!("{:?}", err))});
                  },
                }
              }
              Err(err) => {
                return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1248, kind: MechErrorKind::None});
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
                return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1249, kind: MechErrorKind::None});
              },
            };
          }
          _ => (), // Do nothing if the extension is not recognized
        }
      },
      err => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1250, kind: MechErrorKind::GenericError(format!("{:?}", err))});},
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
            _ => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1241, kind: MechErrorKind::None});},
          }
        }
        _ => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1242, kind: MechErrorKind::None});},
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
        return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1243, kind: MechErrorKind::FileNotFound(path.to_str().unwrap().to_string())});
      }
    };
  }
  Ok(code)
}*/

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
}