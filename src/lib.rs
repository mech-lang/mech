// # Mech

// ## Prelude

extern crate mech_core;
extern crate mech_syntax;
extern crate mech_program;
extern crate mech_utilities;

mod repl;

pub use mech_core::*;
pub use mech_syntax::compiler::*;
pub use mech_program::*;
pub use mech_utilities::*;
pub use self::repl::*;

extern crate colored;
use colored::*;

extern crate bincode;
use std::io::{Write, BufReader, BufWriter};
use std::fs::{OpenOptions, File, canonicalize, create_dir};

extern crate core;
use std::path::{Path, PathBuf};
use std::io;
use std::io::prelude::*;

//extern crate nom;

pub fn read_mech_files(mech_paths: &Vec<String>) -> Result<Vec<MechCode>, MechError> {

  let mut code: Vec<MechCode> = Vec::new();

  let read_file_to_code = |path: &Path| -> Result<Vec<MechCode>, MechError> {
    let mut code: Vec<MechCode> = Vec::new();
    match (path.to_str(), path.extension())  {
      (Some(name), Some(extension)) => {
        match extension.to_str() {
          Some("blx") => {
            match File::open(name) {
              Ok(file) => {
                println!("{} {}", "[Loading]".bright_green(), name);
                let mut reader = BufReader::new(file);
                match bincode::deserialize_from(&mut reader) {
                  Ok(miniblocks) => {code.push(MechCode::MiniBlocks(miniblocks));},
                  Err(err) => {
                    return Err(MechError::GenericError(7492));
                  },
                }
              }
              Err(err) => {
                return Err(MechError::GenericError(7493));
              },
            };
          }
          Some("mec") => {
            match File::open(name) {
              Ok(mut file) => {
                println!("{} {}", "[Loading]".bright_green(), name);
                let mut buffer = String::new();
                file.read_to_string(&mut buffer);
                code.push(MechCode::String(buffer));
              }
              Err(err) => {
                return Err(MechError::GenericError(7494));
              },
            };
          }
          _ => (), // Do nothing if the extension is not recognized
        }
      },
      _ => {return Err(MechError::GenericError(7496));},
    }
    Ok(code)
  };

  for path_str in mech_paths {
    let path = Path::new(path_str);
    // Compile a .mec file on the web
    if path.to_str().unwrap().starts_with("https") {
      println!("{} {}", "[Downloading]".bright_green(), path.display());
      match reqwest::blocking::get(path.to_str().unwrap()) {
        Ok(response) => {
          match response.text() {
            Ok(text) => code.push(MechCode::String(text)),
            _ => {return Err(MechError::GenericError(7497));},
          }
        }
        _ => {return Err(MechError::GenericError(7498));},
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
        return Err(MechError::GenericError(7499));
      }
    };
  }
  Ok(code)
}

pub fn compile_code(code: Vec<MechCode>) -> Result<Vec<MiniBlock>,MechError> {
  println!("{}", "[Compiling] ".bright_green());
  let mut miniblocks = vec![];
  for c in code {
    match c {
      MechCode::String(c) => {
        let mut compiler = Compiler::new();
        let blocks = compiler.compile_str(&c)?;
        let mut mb = minify_blocks(&blocks);
        miniblocks.append(&mut mb);
      },
      MechCode::MiniBlocks(mut mb) => {
        miniblocks.append(&mut mb);
      },
    }
  }
  Ok(miniblocks)
}

pub fn minify_blocks(blocks: &Vec<Block>) -> Vec<MiniBlock> {
  let mut miniblocks = Vec::new();
  for block in blocks {
    let mut miniblock = MiniBlock::new();
    miniblock.transformations = block.transformations.clone();
    match &block.unsatisfied_transformation {
      Some((_,tfm)) => miniblock.transformations.push(tfm.clone()),
      _ => (),
    }
    miniblock.transformations.append(&mut block.pending_transformations.clone());
    /*for (k,v) in block.store.number_literals.iter() {
      miniblock.number_literals.push((k.clone(), v.clone()));
    }
    for error in &block.errors {
      miniblock.errors.push(error.clone());
    }*/
    miniblock.id = block.id;
    miniblocks.push(miniblock);
  }
  miniblocks
}