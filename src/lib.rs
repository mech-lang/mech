// # Mech Program
#![allow(dead_code)]

// ## Prelude
#![feature(extern_prelude)]
#![feature(get_mut_unchecked)]

extern crate core;
extern crate libloading;
extern crate reqwest;

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
extern crate mech_syntax;
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

pub fn download_machine(machine_name: &str, name: &str, path_str: &str, ver: &str, outgoing: Option<crossbeam_channel::Sender<ClientMessage>>) -> Result<Library,Box<dyn std::error::Error>> {
  create_dir("machines");

  let machine_file_path = format!("machines/{}",machine_name);
  {
    let path = Path::new(path_str);
    // Download from the web
    if path.to_str().unwrap().starts_with("https") {
      match outgoing {
        Some(ref sender) => {sender.send(ClientMessage::String(format!("{} {} v{}", "[Downloading]".bright_cyan(), name, ver)));}
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
        Some(sender) => {sender.send(ClientMessage::String(format!("{} {} v{}", "[Loading]".bright_cyan(), name, ver)));}
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
  let machine = unsafe{Library::new(machine_file_path).expect(&message)};
  Ok(machine)
}
