// # Mech

// ## Prelude

extern crate mech_core;
extern crate mech_syntax;
extern crate mech_program;
extern crate mech_utilities;

mod repl;

pub use mech_core::{Core, TableIndex, ValueMethods, Change, Transaction, Transformation, hash_string, Block, Table, Value, Error, ErrorType};
pub use mech_core::QuantityMath;
pub use mech_syntax::compiler::Compiler;
pub use mech_syntax::parser::{Parser, Node as ParserNode};
pub use mech_program::{Program, ProgramRunner, RunLoop, ClientMessage};
pub use mech_utilities::{RunLoopMessage, MiniBlock, MechCode, WebsocketMessage};
pub use self::repl::{ReplCommand, parse_repl_command};


extern crate colored;
use colored::*;

extern crate bincode;
use std::io::{Write, BufReader, BufWriter};
use std::fs::{OpenOptions, File, canonicalize, create_dir};

extern crate core;
use std::path::{Path, PathBuf};
use std::io;
use std::io::prelude::*;

extern crate nom;

pub async fn read_mech_files(mech_paths: Vec<&str>) -> Result<Vec<MechCode>, Box<dyn std::error::Error>> {

  let mut code: Vec<MechCode> = Vec::new();

  for path_str in mech_paths {
    let path = Path::new(path_str);
    // Compile a .mec file on the web
    if path.to_str().unwrap().starts_with("https") {
      println!("{} {}", "[Downloading]".bright_green(), path.display());
      let program = reqwest::get(path.to_str().unwrap()).await?.text().await?;
      code.push(MechCode::String(program));
    } else {
      // Compile a directory of mech files
      if path.is_dir() {
        for entry in path.read_dir().expect("read_dir call failed") {
          if let Ok(entry) = entry {
            match (entry.path().to_str(), entry.path().extension())  {
              (Some(name), Some(extension)) => {
                match extension.to_str() {
                  Some("blx") => {
                    println!("{} {}", "[Loading]".bright_green(), name);
                    let file = File::open(name)?;
                    let mut reader = BufReader::new(file);
                    let miniblocks: Vec<MiniBlock> = bincode::deserialize_from(&mut reader)?;
                    code.push(MechCode::MiniBlocks(miniblocks));
                  }
                  Some("mec") => {
                    println!("{} {}", "[Loading]".bright_green(), name);
                    let mut f = File::open(name)?;
                    let mut buffer = String::new();
                    f.read_to_string(&mut buffer);
                    code.push(MechCode::String(buffer));
                  }
                  _ => (),
                }
              },
              _ => (),
            }
          }
        }
      } else if path.is_file() {
        // Compile a single file
        match (path.to_str(), path.extension())  {
          (Some(name), Some(extension)) => {
            match extension.to_str() {
              Some("blx") => {
                println!("{} {}", "[Loading]".bright_green(), name);
                let file = File::open(name)?;
                let mut reader = BufReader::new(file);
                let miniblocks: Vec<MiniBlock> = bincode::deserialize_from(&mut reader)?;
                code.push(MechCode::MiniBlocks(miniblocks));
              }
              Some("mec") => {
                println!("{} {}", "[Loading]".bright_green(), name);
                let mut f = File::open(name)?;
                let mut buffer = String::new();
                f.read_to_string(&mut buffer);
                code.push(MechCode::String(buffer));
              }
              _ => (),
            }
          },
          _ => (),
        }
      }
    };
  }
  Ok(code)
}

pub fn compile_code(code: Vec<MechCode>) -> Vec<Block> {
  print!("{}", "[Compiling] ".bright_green());
  let mut compiler = Compiler::new();
  for c in code {
    match c {
      MechCode::String(c) => {compiler.compile_string(c);},
      MechCode::MiniBlocks(c) => {
        let mut blocks: Vec<Block> = Vec::new();
        for miniblock in c {
          let mut block = Block::new(100);
          for tfm in miniblock.transformations {
            block.register_transformations(tfm);
          }
          for tfm in miniblock.plan {
            block.plan.push(tfm);
          }
          blocks.push(block);
        }
        compiler.blocks.append(&mut blocks);
      },
    }
  }
  println!("Compiled {} blocks.", compiler.blocks.len());
  compiler.blocks
}

pub fn minify_blocks(blocks: &Vec<Block>) -> Vec<MiniBlock> {
  let mut miniblocks = Vec::new();
  for block in blocks {
    let mut miniblock = MiniBlock::new();
    miniblock.transformations = block.transformations.clone();
    miniblock.plan = block.plan.clone();
    for (k,v) in block.store.strings.iter() {
      miniblock.strings.push((k.clone(), v.clone()));
    }
    for (k,v) in block.store.number_literals.iter() {
      miniblock.number_literals.push((k.clone(), v.clone()));
    }
    miniblocks.push(miniblock);
  }
  miniblocks
}