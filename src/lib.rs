// # Mech Program
#![allow(dead_code)]
#![allow(warnings)]

// ## Prelude
#![feature(extern_prelude)]
#![feature(get_mut_unchecked)]

extern crate core;
extern crate libloading;
extern crate reqwest;
extern crate indexmap;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate serde;
#[macro_use]
extern crate crossbeam_channel;
#[macro_use]
extern crate lazy_static;

extern crate time;

extern crate mech_core;
use mech_core::*;
extern crate mech_syntax;
use mech_syntax::formatter::Formatter;
extern crate mech_utilities;
extern crate colored;
extern crate websocket;

use colored::*;
use libloading::Library;
use std::io::copy;
use std::fs::{OpenOptions, File, canonicalize, create_dir};
use std::path::{Path, PathBuf};
use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;
use reqwest::StatusCode;

// ## Modules

pub mod program;
pub mod persister;
pub mod runloop;

// ## Exported Modules

pub use self::program::{Program};
pub use self::runloop::{ProgramRunner, RunLoop, ClientMessage};
pub use self::persister::{Persister};

pub fn format_errors(errors: &Vec<MechError>) -> String {
  let mut formatted_errors = "".to_string();
  let plural = if errors.len() == 1 {
    ""
  } else {
    "s"
  };
  let error_notice = format!("🐛 Found {} Error{}:\n", &errors.len(), plural);
  formatted_errors = format!("{}\n{}\n\n", formatted_errors, error_notice);
  for error in errors {
    match &error.kind {
      MechErrorKind::ParserError(ast,report,msg) => {
        formatted_errors = format!("{}{}", formatted_errors, msg);
      }
      _ => {
        formatted_errors = format!("{}{} {} {} {}\n\n", formatted_errors, "---".truecolor(246,192,78), "Block".truecolor(246,192,78), "BLOCKNAME", "--------------------------------------------".truecolor(246,192,78));
        formatted_errors = format!("{}\n{:?}\n", formatted_errors, error);
      }
    }
  }
  formatted_errors = format!("{}\n{}",formatted_errors, "----------------------------------------------------------------\n\n".truecolor(246,192,78));
  formatted_errors
}

pub fn download_machine(machine_name: &str, name: &str, path_str: &str, ver: &str, outgoing: Option<crossbeam_channel::Sender<ClientMessage>>) -> Result<Library,MechError> {
  create_dir("machines");

  let machine_file_path = format!("machines/{}",machine_name);
  {
    let path = Path::new(path_str);
    // Download from the web
    if path.to_str().unwrap().starts_with("https") {
      match outgoing {
        Some(ref sender) => {sender.send(ClientMessage::String(format!("{} {} v{}", "[Downloading]".truecolor(153,221,85), name, ver)));}
        None => (),
      }
      let machine_url = format!("{}/{}", path_str, machine_name);
      match reqwest::get(machine_url.as_str()) {
        Ok(mut response) => {
          match response.status() {
            StatusCode::OK => {
              let mut dest = File::create(machine_file_path.clone())?;
              copy(&mut response, &mut dest)?;
            },
            _ => {
              match outgoing {
                Some(sender) => {sender.send(ClientMessage::String(format!("{} Failed to download {} v{}", "[Error]".bright_red(), name, ver)));}
                None => (),
              }
            },
          }
        }
        Err(_) => {
          match outgoing {
            Some(sender) => {sender.send(ClientMessage::String(format!("{} Failed to download {} v{}", "[Error]".bright_red(), name, ver)));}
            None => (),
          }
        }
      }

    // Load from a local directory
    } else {
      match outgoing {
        Some(sender) => {sender.send(ClientMessage::String(format!("{} {} v{}", "[Loading]".truecolor(153,221,85), name, ver)));}
        None => (),
      }
      let machine_path = format!("{}{}", path_str, machine_name);
      let path = Path::new(&machine_path);
      let mut dest = File::create(machine_file_path.clone())?;
      let mut f = File::open(path)?;
      copy(&mut f, &mut dest)?;
    }
  }
  let machine_file_path = format!("machines/{}",machine_name);
  let message = format!("Can't load library {:?}", machine_file_path);
  match unsafe{Library::new(machine_file_path)} {
    Ok(machine) => Ok(machine),
    Err(err) => Err(MechError{id: 1273, kind: MechErrorKind::GenericError(format!("{:?}",message))}),
  }
}
