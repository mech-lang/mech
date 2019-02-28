// # Index

// ## Prelude

use table::{Value, Table, Index};
use alloc::string::String;
use alloc::vec::Vec;
use alloc::fmt;
use hashbrown::hash_map::HashMap;
use hashbrown::hash_set::HashSet;

// ## Hasher

// Hashes strings by breaking them into chunks and adding their byte 
// representations together.

pub struct Hasher {
  value: u64,
}

impl Hasher {

  pub fn new() -> Hasher {
    Hasher {
      value: 0,
    }
  }

  pub fn hash_str(string: &str) -> u64 {
    let mut hasher = Hasher::new();
    hasher.write(string);
    hasher.finish()
  }

  pub fn hash_string(string: String) -> u64 {
    let mut hasher = Hasher::new();
    hasher.write(&string.as_str());
    hasher.finish()
  }

  pub fn hash_byte_vector(bytes: &Vec<u8>) -> u64 {
    let mut hasher = Hasher::new();
    let string;
    unsafe {
      string = String::from_utf8_unchecked(bytes.to_vec());
    }
    hasher.write(&string.as_str());
    hasher.finish()
  }

  pub fn write(&mut self, string: &str) {
    let mult = [1, 256, 65536, 16777216, 1768841549];
    let chunks = CharChunks::new(string, 4);
    for chunk in chunks {
      let byte_string = chunk.as_bytes();
      let mut ix = 0;
      for byte in byte_string {
        self.value = self.value + byte.clone() as u64 * mult[ix];
        ix = ix + 1;
      } 
    }
  }

  pub fn write_value(&mut self, value: &Value) {
    match value {
      &Value::String(ref string) => self.write(&format!("{:?}", string)),
      &Value::Number(ref number) => self.write(&format!("{:?}", number)), 
      _ => (),
    }
  }

  pub fn finish(&mut self) -> u64 {
    let v = self.value;
    self.value = 0;
    v
  }

  pub fn read(&self) -> u64 {
   self.value
  }

  pub fn reset(&mut self) {
    self.value = 0;
  }
} 

// ## Table Index

#[derive(Clone)]
pub struct TableIndex {
  pub map: HashMap<u64, Table>,
  pub aliases: HashMap<u64, u64>,
  pub changed_this_round: HashSet<(u64, Index)>,
}

impl TableIndex {

  pub fn new(capacity: usize) -> TableIndex {
    TableIndex {
      map: HashMap::with_capacity(capacity),
      aliases: HashMap::new(),
      changed_this_round: HashSet::new(),
    }
  }

  pub fn clear(&mut self) {
    self.map.clear();
    self.aliases.clear();
    self.changed_this_round.clear();
  }

  pub fn len(&self) -> usize {
    self.map.len()
  }

  pub fn add_alias(&mut self, table: u64, alias: u64) {
    self.aliases.insert(alias, table);
  }

  pub fn get(&self, table_id: u64) -> Option<&Table> {
    match self.map.get(&table_id) {
      Some(table) => Some(table),
      None => {
        match self.aliases.get(&table_id) {
          Some(id) => self.map.get(&id),
          None => None,
        }
      },
    }
  }

  pub fn get_mut(&mut self, table_id: u64) -> Option<&mut Table> {
    match self.aliases.get(&table_id) {
      Some(id) => self.map.get_mut(&id),
      None => {
        match self.map.get_mut(&table_id) {
          Some(table) => Some(table),
          None => None,
        }
      },
    }
  }

  pub fn insert(&mut self, table: Table) {
    if !self.map.contains_key(&table.id) {
      self.map.insert(table.id, table);
    }
  }

  pub fn contains(&mut self, table: u64) -> bool {
    self.map.contains_key(&table)
  }

  pub fn remove(&mut self, table: &u64) {
    self.map.remove(&table);
  }

}

impl fmt::Debug for TableIndex {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    for (_table_id, table) in self.map.iter() {
      write!(f, "{:?}\n", table).unwrap();
    }
    Ok(())
  }
}


// ## Utility

// Breaks a string into chunks of size n. This code was lifted from the odds 
// crate: https://docs.rs/odds/0.3.1/odds/string/struct.CharChunks.html

#[derive(Clone, Debug)]
struct CharChunks<'a> {
  s: &'a str,
  n: usize,
}

impl<'a> CharChunks<'a> {
  pub fn new(s: &'a str, n: usize) -> Self {
    CharChunks { s: s, n: n }
  }
}

impl<'a> Iterator for CharChunks<'a> {
  type Item = &'a str;
  fn next(&mut self) -> Option<&'a str> {
    let s = self.s;
    if s.is_empty() {
      return None;
    }
    for (i, _) in s.char_indices().enumerate() {
      if i + 1 == self.n {
        let (part, tail) = s.split_at(self.n);
        self.s = tail;
        return Some(part);
      }
    }
    self.s = "";
    Some(s)
  }
}