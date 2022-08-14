#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::{egui};
use eframe::egui::{containers::*, *};
extern crate mech_gui;
extern crate mech_syntax;
extern crate mech_core;

use mech_core::*;
use mech_syntax::compiler::Compiler;

use std::thread::JoinHandle;
extern crate image;
use std::path::Path;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;

use mech_gui::*;

pub fn load_mech(program_path: &str) -> Result<mech_core::Core,MechError> {
  let code_string = match fs::read_to_string(program_path) {
    Ok(code) => code,
    Err(err) => {return Err(MechError{id: 87491, kind: MechErrorKind::GenericError(format!("{:?}",err))});}
  };
  let mut mech_core = mech_core::Core::new();
  let mut compiler = Compiler::new(); 
  match compiler.compile_str(&code_string) {
    Ok(blocks) => {
      mech_core.load_blocks(blocks);
    }
    Err(x) => {
      
    }
  }
  
  let mut code = r#"
#time/timer = [|period<s> ticks<u64>|]
#mech/compiler = [|code<string>| "hi"]
#io/pointer = [|x<f32> y<f32>| 0 0]"#.to_string();
  
  code += r#"
#mech/tables = [|name<string>|
                "time/timer"
                "io/pointer"
                "mech/tables"
                "mech/compiler""#;
  for name in mech_core.table_names() {
  code += &format!("\n{:?}",name);     
  }
  code += "]";
  
  let mut compiler = Compiler::new();
  let blocks = compiler.compile_str(&code).unwrap();
  mech_core.load_blocks(blocks);
  mech_core.schedule_blocks();
  Ok(mech_core)
}

fn main() {
  //let input = std::env::args().nth(1).unwrap();
  let mut native_options = eframe::NativeOptions::default();
  let path = concat!(env!("CARGO_MANIFEST_DIR"), "/mech.ico");
  let icon = load_icon(Path::new(path));
  let core = load_mech(r#"src\bin\notebook.mec"#).unwrap();
  native_options.icon_data = Some(icon);
  native_options.min_window_size = Some(Vec2{x: 1480.0, y: 800.0});
  eframe::run_native("Mech Notebook", native_options, Box::new(|cc| 
    Box::new(MechApp::new(cc,core))));
}


