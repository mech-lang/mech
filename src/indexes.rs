// # Index

// ## Prelude

use table::{Value, Table};
use alloc::{fmt};
use hashmap_core::map::HashMap;

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
        hasher.write(&string.to_owned());
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
            &Value::String(ref string) => self.write(&string),
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

pub struct TableIndex {
    pub name_map: HashMap<u64, String>,
    pub map: HashMap<u64, (Table, Vec<(u64, u64, usize)>)>,
}

impl TableIndex {

    pub fn new(capacity: usize) -> TableIndex {
        TableIndex {
            name_map: HashMap::with_capacity(capacity),
            map: HashMap::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn get(&self, table_id: u64) -> Option<&Table> {
        match self.map.get(&table_id) {
            Some((table, _)) => {
                Some(table)
            },
            None => None,
        }
    }

    pub fn get_mut(&mut self, table_id: u64) -> Option<&mut Table> {
        match self.map.get_mut(&table_id) {
            Some((table, _)) => {
                Some(table)
            },
            None => None,
        }
    }

    pub fn register(&mut self, table: Table) {
        if !self.map.contains_key(&table.id) {
            self.map.insert(table.id, (table, Vec::with_capacity(100)));
        }
    }

    pub fn remove(&mut self, table: &u64) {
        self.map.remove(table);
    }

}

impl fmt::Debug for TableIndex {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (table_id, _) in self.map.iter() {
            write!(f, "{:#x}\n", table_id).unwrap();
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