use mech_core::{Core, humanize, Register, Transaction, Change};
use std::fs::{OpenOptions, File, canonicalize, create_dir};
use std::io::{Write, BufReader, BufWriter};
use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;
use std::thread::{self, JoinHandle};
use std::mem;

// ## Persister

pub enum PersisterMessage {
  Stop,
  Write(Vec<Change>),
}

pub struct Persister {
  thread: JoinHandle<()>,
  outgoing: Sender<PersisterMessage>,
  loaded: Vec<Change>,
}

impl Persister {
  pub fn new(path_ref:&str) -> Persister {
    let (outgoing, incoming) = crossbeam_channel::unbounded();
    let path = path_ref.to_string();
    let thread = thread::spawn(move || {
      let file = OpenOptions::new().append(true).create(true).open(&path).unwrap();
      let mut writer = BufWriter::new(file);
      loop {
        match incoming.recv().unwrap() {
          PersisterMessage::Stop => { break; }
          PersisterMessage::Write(items) => {
            for item in items {
              let result = bincode::serialize(&item).unwrap();
              if let Err(e) = writer.write_all(&result) {
                panic!("Can't persist! {:?}", e);
              }
            }
            writer.flush().unwrap();
          }
        }
      }
    });
    Persister { outgoing, thread, loaded: vec![] }
  }

  pub fn load(&mut self, path: &str) {
    let file = match File::open(path) {
      Ok(f) => f,
      Err(_) => {
        ////println!("Unable to load db: {}", path);
        return;
      }
    };
    let mut reader = BufReader::new(file);
    loop {
      let result:Result<Change, _> = bincode::deserialize_from(&mut reader);
      match result {
        Ok(change) => {
          self.loaded.push(change);
        },
        Err(info) => {
          ////println!("ran out {:?}", info);
          break;
        }
      }
    }
  }

  pub fn send(&self, changes: Vec<Change>) {
    self.outgoing.send(PersisterMessage::Write(changes)).unwrap();
  }

  pub fn wait(self) {
    self.thread.join().unwrap();
  }

    pub fn get_channel(&self) -> Sender<PersisterMessage> {
    self.outgoing.clone()
  }

  pub fn get_changes(&mut self) -> Vec<Change> {
    mem::replace(&mut self.loaded, vec![])
  }

  pub fn close(&self) {
    self.outgoing.send(PersisterMessage::Stop).unwrap();
  }
}