extern crate crossbeam_channel;
use mech_core::*;
use mech_utilities::*;
//use std::sync::mpsc::{self, Sender};
use std::thread::{self};
use crossbeam_channel::Sender;
use std::collections::HashMap;
use std::io;
use std::io::prelude::*;

lazy_static! {
  static ref IO_OUT: u64 = hash_str("io/out");
  static ref TEXT: u64 = hash_str("text");
  static ref COLOR: u64 = hash_str("color");
}

export_machine!(io_out, io_out_reg);

extern "C" fn io_out_reg(registrar: &mut dyn MachineRegistrar, outgoing: Sender<RunLoopMessage>) -> String {
  registrar.register_machine(Box::new(Out{outgoing}));
  "#io/out = [|text<string> color<u32>|]".to_string()
}

#[derive(Debug)]
pub struct Out {
  outgoing: Sender<RunLoopMessage>,
  //printed: usize,
}

impl Machine for Out {

  fn name(&self) -> String {
    "io/out".to_string()
  }

  fn id(&self) -> u64 {
    hash_str(&self.name())
  }

  fn on_change(&mut self, table: &Table) -> Result<(), MechError> {
    for i in 1..=table.rows {
      let text = table.get(&TableIndex::Index(i),&TableIndex::Alias(*TEXT))?;
      let color = table.get(&TableIndex::Index(i),&TableIndex::Alias(*COLOR))?;
      match (text,color) {
        (Value::String(string),_) => {
          println!("{:?}", string);
        }
        (Value::Bool(truth),_) => {
          println!("{:?}", truth);
        }
        x => {return Err(MechError{id: 8395, kind: MechErrorKind::GenericError(format!("{:?}",x))})},
      }
    }
    //table.clear();
    Ok(())
  }
}

