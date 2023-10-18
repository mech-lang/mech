extern crate crossbeam_channel;
extern crate reqwest;
use mech_core::*;
use mech_utilities::*;
use std::thread::{self};
use crossbeam_channel::Sender;
use std::collections::HashMap;

lazy_static! {
  static ref HTTP_REQUEST: u64 = hash_str("http/request");
  static ref URI: u64 = hash_str("uri");
  static ref HEADER: u64 = hash_str("header");
  static ref RESPONSE: u64 = hash_str("response");
}

export_machine!(http_request, http_request_reg);

extern "C" fn http_request_reg(registrar: &mut dyn MachineRegistrar, outgoing: Sender<RunLoopMessage>) -> String {
  registrar.register_machine(Box::new(Request{outgoing}));
  "#http/request = [|uri<string> header<string> response<string>|]".to_string()
}

#[derive(Debug)]
pub struct Request {
  outgoing: Sender<RunLoopMessage>,
}

impl Machine for Request {

  fn name(&self) -> String {
    "http/request".to_string()
  }

  fn id(&self) -> u64 {
    hash_str(&self.name())
  }

  fn on_change(&mut self, table: &Table) -> Result<(), MechError> {
    for i in 1..=table.rows {
      let row = TableIndex::Index(i);
      let uri = table.get(&row,&TableIndex::Alias(*URI))?;
      match uri {
        Value::String(uri) => {
          let outgoing = self.outgoing.clone();
          let uri = uri.clone();
          let request_handle = thread::spawn(move || {
            match reqwest::blocking::get(uri.to_string()) {
              Ok(response) => {
                if response.status().is_success() {
                  let text = response.text().unwrap();
                  outgoing.send(RunLoopMessage::Transaction(vec![
                    Change::Set((*HTTP_REQUEST, vec![(row, TableIndex::Alias(*RESPONSE), Value::from_string(&text))])),
                  ]));
                } else if response.status().is_server_error() {
                  // TODO Handle Error
                } else {
                  // TODO Handle Error
                }
              }
              Err(_) => (), // TODO Handle errors
            }
          });
        }
        _ => (), // TODO Send error
      }
    }
    Ok(())
  }
}