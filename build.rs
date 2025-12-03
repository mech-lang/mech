#![allow(warnings)]
use std::{
  env,
  error::Error,
  fs::{self, File},
  io::Write,
  path::Path,
};
extern crate winres;

const SOURCE_DIR: &str = r"project";

fn main() -> Result<(), Box<dyn Error>> {
  
  if cfg!(target_os = "windows") {
    let mut res = winres::WindowsResource::new();
    res.set_icon("mech.ico");
    res.compile().unwrap();
  }

  if Path::new("src/wasm/pkg/mech_wasm_bg.wasm.br").exists() {
    println!("cargo:rustc-cfg=has_file_wasm");
  }

  if Path::new("src/wasm/pkg/mech_wasm.js").exists() {
    println!("cargo:rustc-cfg=has_file_js");
  }

  if Path::new("include/index.html").exists() {
    println!("cargo:rustc-cfg=has_file_shim");
  }

  if Path::new("include/style.css").exists() {
    println!("cargo:rustc-cfg=has_file_stylesheet");
  }

  Ok(())
}