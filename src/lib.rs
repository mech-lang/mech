// # Mech

// ## Prelude

extern crate mech_core;
extern crate mech_syntax;
extern crate mech_program;
extern crate mech_utilities;

mod repl;

pub use mech_core::*;
pub use mech_syntax::*;
pub use mech_program::*;
pub use mech_utilities::*;
//pub use self::repl::{ReplCommand, parse_repl_command};

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

pub fn read_mech_files(mech_paths: &Vec<String>) -> Result<Vec<MechCode>, Box<dyn std::error::Error>> {

  let mut code: Vec<MechCode> = Vec::new();

  let read_file_to_code = |path: &Path| -> Vec<MechCode> {
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
                  Ok(miniprograms) => {code.push(MechCode::MiniPrograms(miniprograms));},
                  Err(err) => {
                    println!("{} Failed to load {}", "[Error]".bright_red(), name);
                  },
                }
              }
              Err(err) => {
                println!("{} Failed to load {}", "[Error]".bright_red(), name);
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
                println!("{} Failed to load {}", "[Error]".bright_red(), name);
              },
            };
          }
          _ => (),
        }
      },
      _ => {println!("{} Failed to load {:?}", "[Error]".bright_red(), path);},
    }
    code
  };

  for path_str in mech_paths {
    let path = Path::new(path_str);
    // Compile a .mec file on the web
    if path.to_str().unwrap().starts_with("https") {
      println!("{} {}", "[Downloading]".bright_green(), path.display());
      let program = reqwest::blocking::get(path.to_str().unwrap())?.text()?;
      code.push(MechCode::String(program));
    } else {
      // Compile a directory of mech files
      if path.is_dir() {
        for entry in path.read_dir().expect("read_dir call failed") {
          if let Ok(entry) = entry {
            let path = entry.path();
            let mut new_code = read_file_to_code(&path);
            code.append(&mut new_code);
          }
        }
      } else if path.is_file() {
        // Compile a single file
        let mut new_code = read_file_to_code(&path);
        code.append(&mut new_code);
      } else {
        println!("{} Failed to open {:?}", "[Error]".bright_red(), path);
      }
    };
  }
  Ok(code)
}

pub fn compile_code(code: Vec<MechCode>) -> Vec<MiniProgram> {
  print!("{}", "[Compiling] ".bright_green());
  let mut miniprograms = vec![];
  for c in code {
    match c {
      MechCode::String(c) => {
        //let mut compiler = Compiler::new();
        //let programs = compiler.compile_string(c);
        //let mut mp = programs.iter().map(|p| minify_program(p)).collect::<Vec<MiniProgram>>();
        //miniprograms.append(&mut mp);
      },
      MechCode::MiniBlocks(miniblocks) => {
        miniprograms.push(MiniProgram{title: None, blocks: miniblocks});
      },
      MechCode::MiniPrograms(mut p) => {
        miniprograms.append(&mut p);
      }
    }
  }
  miniprograms
}

pub fn minify_blocks(blocks: &Vec<Block>) -> Vec<MiniBlock> {
  let mut miniblocks = Vec::new();
  for block in blocks {
    let mut miniblock = MiniBlock::new();
    /*miniblock.transformations = block.transformations.clone();
    miniblock.plan = block.plan.clone();
    for (k,v) in block.store.strings.iter() {
      miniblock.strings.push((k.clone(), v.clone()));
    }
    for (k,v) in block.store.number_literals.iter() {
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

/*
pub fn minify_program(program: &mech_syntax::compiler::Program) -> MiniProgram {
  let mut miniblocks = Vec::new();
  for block in &program.blocks {
    let mut miniblock = MiniBlock::new();
    /*miniblock.transformations = block.transformations.clone();
    miniblock.plan = block.plan.clone();
    for (k,v) in block.store.strings.iter() {
      miniblock.strings.push((k.clone(), v.clone()));
    }
    for (k,v) in block.store.number_literals.iter() {
      miniblock.number_literals.push((k.clone(), v.clone()));
    }
    for error in &block.errors {
      miniblock.errors.push(error.clone());
    }*/
    miniblock.id = block.id;
    miniblocks.push(miniblock);
  }
  MiniProgram{title: program.title.clone(), blocks: miniblocks}
}*/