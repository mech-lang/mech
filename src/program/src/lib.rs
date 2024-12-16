// # Mech Program
#![allow(dead_code)]
#![allow(warnings)]

// ## Prelude
#![feature(extern_prelude)]
#![feature(get_mut_unchecked)]
#![feature(hash_extract_if)]


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
use mech_utilities::{RunLoopMessage, MechSocket, SocketMessage};

use std::io;
use std::io::prelude::*;
use std::time::{Duration, Instant, SystemTime};
use std::thread::{self, JoinHandle};
use std::sync::Mutex;
use websocket::sync::Server;
use std::net::{SocketAddr, UdpSocket, TcpListener, TcpStream};
use std::collections::HashMap;

// ## Modules

pub mod program;
pub mod persister;
pub mod runloop;

// ## Exported Modules

pub use self::program::{Program};
pub use self::runloop::{ProgramRunner, RunLoop, ClientMessage};
pub use self::persister::{Persister};

lazy_static! {
  static ref CORE_MAP: Mutex<HashMap<SocketAddr, (String, SystemTime)>> = Mutex::new(HashMap::new());
}

pub fn start_maestro(mech_socket_address: String, formatted_address: String, maestro_address: String, websocket_address: String, mech_client_channel: Sender<RunLoopMessage>) -> Result<JoinHandle<()>,MechError> {

  mech_client_channel.send(RunLoopMessage::String((format!("Core socket started at: {}", mech_socket_address.clone()),None)));

  let mech_client_channel_ws = mech_client_channel.clone();
  let mech_client_channel_heartbeat = mech_client_channel.clone();

  let core_thread = thread::Builder::new().name("Core socket".to_string()).spawn(move || {
    // A socket bound to 3235 is the maestro. It will be the one other cores search for
    'socket_loop: loop {
      match UdpSocket::bind(maestro_address.clone()) {
        // The maestro core
        Ok(socket) => {
          mech_client_channel.send(RunLoopMessage::String((format!("{} Socket started at: {}", "[Maestro]".truecolor(246,192,78), maestro_address),None)));
          let mut buf = [0; 16_383];
          // Heartbeat thread periodically checks to see how long it's been since we've last heard from each remote core
          thread::Builder::new().name("Heartbeat".to_string()).spawn(move || {
            loop {
              thread::sleep(Duration::from_millis(500));
              let now = SystemTime::now();
              let mut core_map = CORE_MAP.lock().unwrap();
              // If a core hasn't been heard from since 1 second ago, disconnect it.
              for (_, (remote_core_address, _)) in core_map.extract_if(|_k,(_, last_seen)| now.duration_since(*last_seen).unwrap().as_secs_f32() > 1.0) {
                mech_client_channel_heartbeat.send(RunLoopMessage::RemoteCoreDisconnect(hash_str(&remote_core_address.to_string())));
              }
            }
          });
          // TCP socket thread for websocket connections
          thread::Builder::new().name("TCP Socket".to_string()).spawn(move || {
            let server = Server::bind(websocket_address.clone()).unwrap();
            mech_client_channel_ws.send(RunLoopMessage::String((format!("{} Websocket server started at: {}","[Maestro]".truecolor(246,192,78), &websocket_address),None)));
            for request in server.filter_map(Result::ok) {
              let mut ws_stream = request.accept().unwrap();
              let address = ws_stream.peer_addr().unwrap();
              mech_client_channel_ws.send(RunLoopMessage::RemoteCoreConnect(MechSocket::WebSocket(ws_stream)));
            }
          });

          // Loop to receive UDP messages from remote cores
          loop {
            let (amt, src) = socket.recv_from(&mut buf).unwrap();
            let now = SystemTime::now();
            let message: Result<SocketMessage, bincode::Error> = bincode::deserialize(&buf);
            match message {
              // If a remote core connects, send a connection message back to it
              Ok(SocketMessage::RemoteCoreConnect(remote_core_address)) => {
                CORE_MAP.lock().unwrap().insert(src,(remote_core_address.clone(), SystemTime::now()));
                mech_client_channel.send(RunLoopMessage::RemoteCoreConnect(MechSocket::UdpSocket(remote_core_address)));
                let message = bincode::serialize(&SocketMessage::RemoteCoreConnect(mech_socket_address.clone())).unwrap();
                let len = socket.send_to(&message, src.clone()).unwrap();
              },
              Ok(SocketMessage::Ping) => {
                let mut core_map = CORE_MAP.lock().unwrap();
                match core_map.get_mut(&src) {
                  Some((_, last_seen)) => {
                    *last_seen = now;
                  } 
                  None => (),
                }
                let message = bincode::serialize(&SocketMessage::Pong).unwrap();
                let len = socket.send_to(&message, src).unwrap();
              },
              _ => (),
            }
          }
        }
        // Maestro port is already bound, start a remote core
        Err(_) => {
          let socket = UdpSocket::bind(formatted_address.clone()).unwrap();
          let message = bincode::serialize(&SocketMessage::RemoteCoreConnect(mech_socket_address.clone().to_string())).unwrap();
          // Send a remote core message to the maestro
          let len = socket.send_to(&message, maestro_address.clone()).unwrap();
          let mut buf = [0; 16_383];
          loop {
            let message = bincode::serialize(&SocketMessage::Ping).unwrap();
            let len = socket.send_to(&message, maestro_address.clone()).unwrap();
            match socket.recv_from(&mut buf) {
              Ok((amt, src)) => {
                let now = SystemTime::now();
                if src.to_string() == maestro_address {
                  let message: Result<SocketMessage, bincode::Error> = bincode::deserialize(&buf);
                  match message {
                    Ok(SocketMessage::Pong) => {
                      thread::sleep(Duration::from_millis(500));
                      // Maestro is still alive
                    },
                    Ok(SocketMessage::RemoteCoreConnect(remote_core_address)) => {
                      CORE_MAP.lock().unwrap().insert(src,(remote_core_address.clone(), SystemTime::now()));
                      mech_client_channel.send(RunLoopMessage::RemoteCoreConnect(MechSocket::UdpSocket(remote_core_address)));
                    }
                    _ => (),
                  }
                }
              } 
              Err(_) => {
                mech_client_channel_ws.send(RunLoopMessage::String(("Maestro has died.".to_string(),None)));
                continue 'socket_loop;
              }
            }
          }
        }
      }
    }
  }).unwrap();
  Ok(core_thread)
}

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
    formatted_errors = format!("{}{}\n\n", formatted_errors, "───────────────────────────────────────────────────────────────────".truecolor(246,192,78));
    match &error.kind {
      MechErrorKind::ParserError(ast,report,msg) => { formatted_errors = format!("{}{}", formatted_errors, msg);}
      MechErrorKind::MissingTable(table_id) => {
        formatted_errors = format!("{} Missing table: {}\n", formatted_errors, error.msg);
      }
      _ => {
        formatted_errors = format!("{}\n{:?}\n", formatted_errors, error);
      }
    }
  }
  formatted_errors = format!("{}\n{}",formatted_errors, "───────────────────────────────────────────────────────────────────\n\n".truecolor(246,192,78));
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
    Err(err) => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1273, kind: MechErrorKind::GenericError(format!("{:?}",message))}),
  }
}
