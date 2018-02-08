// # Index

// ## Prelude

use eav::{Entity, Attribute, Value};

// ## Hasher

pub struct Hasher {
    value: u64,
}

impl Hasher {

    pub fn new() -> Hasher {
        Hasher {
            value: 0,
        }
    }

    pub fn hash_entity(&mut self, entity: Entity) {

    }

    

    pub fn write(&mut self, string: &str) {
        let intLength = string.len() / 4;
        let mult = [1, 256, 65536, 16777216, 1768841549, 4294967296];
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

    pub fn finish(&self) -> u64 {
        self.value
    }


    pub fn reset(&mut self) {
        self.value = 0;
    }

} 




#[derive(Clone, Debug)]
pub struct CharChunks<'a> {
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
        for (i, (j, ch)) in s.char_indices().enumerate() {
            if i + 1 == self.n {
                // FIXME: Use .split_at() when rust version allows
                let mid = j + ch.len_utf8();
                let (part, tail) = (&s[..mid], &s[mid..]);
                self.s = tail;
                return Some(part);
            }
        }
        self.s = "";
        Some(s)
    }
}