// # Mech Server

/*
 Mech Server is a wrapper around the mech runtime. It provides interfaces for 
 controlling the runtime, sending it transactions, and responding to changes.
*/

// ## Prelude

extern crate core;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};

extern crate clap;
use clap::{Arg, App};

extern crate mech_server;
use mech_server::program::{ProgramRunner, RunLoop, RunLoopMessage};
use mech_server::watchers::system::{SystemTimerWatcher};
use mech_server::watchers::websocket::{WebsocketClientWatcher};
use mech_server::client::ClientHandler;


// ## Server Entry

fn main() {

  let matches = App::new("Mech Server")
    .version("0.0.1")
    .author("Corey Montella")
    .about("Creates an instance of a Mech server. Default values for options are in parentheses.")
    .arg(Arg::with_name("mech_file_paths")
      .help("The files and folders from which to load .mec files")
      .required(false)
      .multiple(true))
    .arg(Arg::with_name("port")
      .short("p")
      .long("port")
      .value_name("PORT")
      .help("Sets the port for the Mech server (3012)")
      .takes_value(true))
    .arg(Arg::with_name("http-port")
      .short("t")
      .long("http-port")
      .value_name("PORT")
      .help("Sets the port for the HTTP server (8081)")
      .takes_value(true))
    .arg(Arg::with_name("address")
      .short("a")
      .long("address")
      .value_name("ADDRESS")
      .help("Sets the address of the server (127.0.0.1)")
      .takes_value(true))
    .arg(Arg::with_name("persist")
      .short("s")
      .long("persist")
      .value_name("FILE")
      .help("Sets the name for the database to load from and write to")
      .takes_value(true))
    .get_matches();

  let wport = matches.value_of("port").unwrap_or("3012");
  let hport = matches.value_of("http-port").unwrap_or("8081");
  let address = matches.value_of("address").unwrap_or("127.0.0.1");
  let http_address = format!("{}:{}",address,hport);
  let websocket_address = format!("{}:{}",address,wport);
  let mech_paths = matches.values_of("mech_file_paths").map_or(vec![], |files| files.collect());

  mech_server::http_server(http_address);
  mech_server::websocket_server(websocket_address, mech_paths);
}