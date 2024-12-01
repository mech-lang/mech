#![feature(get_mut_unchecked)]
#![feature(concat_idents)]
#![allow(warnings)]
#![feature(iter_intersperse)]
#![feature(extract_if)]
#![cfg_attr(feature = "no-std", no_std)]
#![cfg_attr(feature = "no-std", alloc)]
#![allow(dead_code)]
#![feature(step_trait)]

#[cfg(feature="no-std")] #[macro_use] extern crate alloc;
#[cfg(not(feature = "no-std"))] extern crate core;
#[macro_use]
extern crate lazy_static;
extern crate nalgebra as na;
extern crate tabled;
extern crate libm;

#[cfg(feature = "no-std")]
extern crate alloc;
extern crate core as rust_core;
extern crate hashbrown;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate num_traits;
extern crate ed25519_dalek;
extern crate rand;
extern crate getrandom;

extern crate seahash;
extern crate indexmap;
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;
use num_traits::*;
use std::ops::*;

pub mod value;
pub mod matrix;
pub mod types;
pub mod functions;
pub mod kind;
pub mod error;
pub mod nodes;

pub use self::value::*;
pub use self::matrix::*;
pub use self::types::*;
pub use self::functions::*;
pub use self::kind::*;
pub use self::error::*;
pub use self::nodes::*;

pub fn hash_chars(input: &Vec<char>) -> u64 {
  seahash::hash(input.iter().map(|s| String::from(*s)).collect::<String>().as_bytes()) & 0x00FFFFFFFFFFFFFF
}

pub fn hash_bytes(input: &Vec<u8>) -> u64 {
  seahash::hash(input) & 0x00FFFFFFFFFFFFFF
}

pub fn hash_str(input: &str) -> u64 {
  seahash::hash(input.to_string().as_bytes()) & 0x00FFFFFFFFFFFFFF
}

pub fn humanize(hash: &u64) -> String {
  let bytes: [u8; 8] = hash.to_be_bytes();
  let mut string = "".to_string();
  let mut ix = 0;
  for byte in bytes.iter() {
    if ix % 2 == 0 {
      ix += 1;
      continue;
    }
    string.push_str(&WORDLIST[*byte as usize]);
    if ix < 7 {
      string.push_str("-");
    }
    ix += 1;
  }
  string
}


pub const WORDLIST: &[&str;256] = &[
  "nil", "ama", "ine", "ska", "pha", "gel", "art", 
  "ona", "sas", "ist", "aus", "pen", "ust", "umn",
  "ado", "con", "loo", "man", "eer", "lin", "ium",
  "ack", "som", "lue", "ird", "avo", "dog", "ger",
  "ter", "nia", "bon", "nal", "ina", "pet", "cat",
  "ing", "lie", "ken", "fee", "ola", "old", "rad",
  "met", "cut", "azy", "cup", "ota", "dec", "del",
  "elt", "iet", "don", "ble", "ear", "rth", "eas", 
  "war", "eig", "tee", "ele", "emm", "ene", "qua",
  "tst", "fan", "fif", "fil", "fin", "fis", "fiv", 
  "flo", "for", "foo", "fou", "fot", "fox", "fre",
  "fri", "fru", "gee", "gia", "glu", "fol", "gre", 
  "ham", "hap", "har", "haw", "hel", "hig", "hot", 
  "hyd", "ida", "ill", "ind", "ini", "ink", "iwa",
  "and", "ite", "jer", "jig", "joh", "jul", "uly", 
  "kan", "ket", "kil", "kin", "kit", "lac", "lak", 
  "lem", "ard", "lim", "lio", "lit", "lon", "lou",
  "low", "mag", "nes", "mai", "gam", "arc", "mar",
  "mao", "mas", "may", "mex", "mic", "mik", "ril",
  "min", "mir", "mis", "mio", "mob", "moc", "ech",
  "moe", "tan", "oon", "ain", "mup", "sic", "neb",
  "une", "net", "nev", "nin", "een", "nit", "nor",
  "nov", "nut", "oct", "ohi", "okl", "one", "ora",
  "ges", "ore", "osc", "ove", "oxy", "pap", "par", 
  "pey", "pip", "piz", "plu", "pot", "pri", "pur",
  "que", "uqi", "qui", "red", "riv", "rob", "roi", 
  "rug", "sad", "sal", "sat", "sep", "sev", "eve",
  "sha", "sie", "sin", "sik", "six", "sit", "sky", 
  "soc", "sod", "sol", "sot", "tir", "ker", "spr",
  "sta", "ste", "mam", "mer", "swe", "tab", "tag", 
  "see", "nis", "tex", "thi", "the", "tim", "tri",
  "twe", "ent", "two", "unc", "ess", "uni", "ura", 
  "veg", "ven", "ver", "vic", "vid", "vio", "vir",
  "was", "est", "whi", "hit", "iam", "win", "his",
  "wis", "olf", "wyo", "ray", "ank", "yel", "zeb",
  "ulu", "fix", "gry", "hol", "jup", "lam", "pas",
  "rom", "sne", "ten", "uta"];