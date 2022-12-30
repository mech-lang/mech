#![allow(warnings)]
#![feature(hash_drain_filter)]
// # Mech

// ## Prelude

pub extern crate mech_core as core;
pub extern crate mech_syntax as syntax;
pub extern crate mech_program as program;
pub extern crate mech_utilities as utilities;

mod repl;

pub use mech_core::*;
pub use mech_syntax::compiler::*;
pub use mech_program::*;
pub use mech_utilities::*;
pub use self::repl::*;

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
use websocket::sync::Server;
use std::net::{SocketAddr, UdpSocket, TcpListener, TcpStream};
use std::collections::HashMap;
use crossbeam_channel::Sender;
#[macro_use]
extern crate lazy_static;

lazy_static! {
  static ref CORE_MAP: Mutex<HashMap<SocketAddr, (String, SystemTime)>> = Mutex::new(HashMap::new());
}

//extern crate nom;

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
              for (_, (remote_core_address, _)) in core_map.drain_filter(|_k,(_, last_seen)| now.duration_since(*last_seen).unwrap().as_secs_f32() > 1.0) {
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
                println!("{} {}", "[Loading]".truecolor(153,221,85), name);
                let mut reader = BufReader::new(file);
                match bincode::deserialize_from(&mut reader) {
                  Ok(miniblocks) => {code.push(MechCode::MiniBlocks(miniblocks));},
                  Err(err) => {
                    return Err(MechError{id: 1247, kind: MechErrorKind::GenericError(format!("{:?}", err))});
                  },
                }
              }
              Err(err) => {
                return Err(MechError{id: 1248, kind: MechErrorKind::None});
              },
            };
          }
          Some("mec") => {
            match File::open(name) {
              Ok(mut file) => {
                println!("{} {}", "[Loading]".truecolor(153,221,85), name);
                let mut buffer = String::new();
                file.read_to_string(&mut buffer);
                code.push(MechCode::String(buffer));
              }
              Err(err) => {
                return Err(MechError{id: 1249, kind: MechErrorKind::None});
              },
            };
          }
          _ => (), // Do nothing if the extension is not recognized
        }
      },
      _ => {return Err(MechError{id: 1250, kind: MechErrorKind::None});},
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
            Ok(text) => code.push(MechCode::String(text)),
            _ => {return Err(MechError{id: 1241, kind: MechErrorKind::None});},
          }
        }
        _ => {return Err(MechError{id: 1242, kind: MechErrorKind::None});},
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
        return Err(MechError{id: 1243, kind: MechErrorKind::FileNotFound(path.to_str().unwrap().to_string())});
      }
    };
  }
  Ok(code)
}

pub fn compile_code(code: Vec<MechCode>) -> Result<Vec<Vec<MiniBlock>>,MechError> {
  print!("{}", "[Compiling] ".truecolor(153,221,85));
  stdout().flush();
  let mut sections = vec![];
  let now = Instant::now();
  for c in code {
    match c {
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