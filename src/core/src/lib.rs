#![cfg_attr(feature = "no_std", no_std)]
#![feature(get_mut_unchecked)]
#![allow(warnings)]
#![feature(iter_intersperse)]
#![feature(extract_if)]
#![allow(dead_code)]
#![feature(step_trait)]
#![feature(box_patterns)]
#![feature(where_clause_attrs)]

//extern crate core as rust_core;
extern crate seahash;

#[cfg(feature="no_std")] #[macro_use] 
extern crate alloc;
#[cfg(not(feature = "no_std"))] 
extern crate core;

#[cfg(feature="no_std")]
use hashbrown::HashMap;
#[cfg(not(feature = "no_std"))] 
use std::collections::HashMap;

#[cfg(feature="no_std")]
use alloc::fmt::{self, Debug, Display};
#[cfg(not(feature = "no_std"))] 
use std::fmt::{self, Debug, Display};

#[cfg(feature="no_std")]
use alloc::vec::Vec;

#[cfg(feature="no_std")]
use fxhash::FxHasher;

#[cfg(feature = "no_std")]
use embedded_io::{self, Read, Write};
#[cfg(not(feature = "no_std"))] 
use std::io::{self, Error as ioError, Cursor, Read, Write};

#[cfg(feature = "no_std")]
use alloc::string::{String, ToString};

#[cfg(feature = "no_std")]
use core::hash::{Hash, Hasher};
#[cfg(not(feature = "no_std"))] 
use std::hash::{Hash, Hasher};

#[cfg(feature = "no_std")]
use alloc::boxed::Box;

#[cfg(feature = "matrix")]  
extern crate nalgebra as na;
#[cfg(feature = "pretty_print")]
extern crate tabled;
#[cfg(feature = "serde")] #[macro_use] 
extern crate serde_derive;
#[cfg(feature = "serde")]
extern crate serde;
#[cfg(any(feature = "math_exp", feature = "f64", feature = "f32", feature = "complex", feature = "rational"))]
extern crate num_traits;

use paste::paste;

#[cfg(any(feature = "math_exp", feature = "f64"))]
use num_traits::*;
#[cfg(feature = "rational")]
use num_rational::Rational64;
#[cfg(feature = "vector3")]
use nalgebra::Vector3;
#[cfg(feature = "vectord")]
use nalgebra::DVector;
#[cfg(feature = "vector2")]
use nalgebra::Vector2;
#[cfg(feature = "vector4")]
use nalgebra::Vector4;
#[cfg(feature = "row_vectord")]
use nalgebra::RowDVector;
#[cfg(feature = "matrix1")]
use nalgebra::Matrix1;
#[cfg(feature = "matrix3")]
use nalgebra::Matrix3;
#[cfg(feature = "matrix4")]
use nalgebra::Matrix4;
#[cfg(feature = "row_vector3")]
use nalgebra::RowVector3;
#[cfg(feature = "row_vector4")]
use nalgebra::RowVector4;
#[cfg(feature = "row_vector2")]
use nalgebra::RowVector2;
#[cfg(feature = "matrixd")]
use nalgebra::DMatrix;
#[cfg(feature = "matrix2x3")]
use nalgebra::Matrix2x3;
#[cfg(feature = "matrix3x2")]
use nalgebra::Matrix3x2;
#[cfg(feature = "matrix2")]
use nalgebra::Matrix2;

#[cfg(feature = "pretty_print")]
use tabled::{
  builder::Builder,
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};

pub mod error;
pub mod kind;
pub mod nodes;
pub mod structures;
pub mod value;
#[cfg(feature = "functions")]
pub mod functions;
pub mod program;
pub mod stdlib;
pub mod types;

pub use self::error::*;
pub use self::kind::*;
pub use self::nodes::*;
pub use self::structures::*;
pub use self::value::*;
#[cfg(feature = "functions")]
pub use self::functions::*;
pub use self::program::*;
pub use self::stdlib::*;
pub use self::types::*;

// Mech Source Code
// ---------------------------------------------------------------------------

#[cfg(feature = "functions")]
inventory::collect!(FunctionDescriptor);

#[cfg(feature = "functions")]
inventory::collect!(FunctionCompilerDescriptor);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MechSourceCode {
  String(String),
  Tree(Program),
  Html(String),
  ByteCode(Vec<u8>),
  Program(Vec<MechSourceCode>),
  Image(Vec<u8>),
}

impl MechSourceCode {

  pub fn to_string(&self) -> String {
    match self {
      MechSourceCode::ByteCode(bc) => {
        #[cfg(feature = "program")]
        match ParsedProgram::from_bytes(bc) {
          Ok(program) => {
            format!("{:#?}",program)
          },
          Err(e) => return format!("Error parsing bytecode: {:?}", e),
        }
        #[cfg(not(feature = "program"))]
        format!("{:#?}", bc)
        
      }
      MechSourceCode::String(s) => s.clone(),
      MechSourceCode::Tree(p) => todo!("Print the tree!"),
      MechSourceCode::Html(h) => h.clone(),
      MechSourceCode::Program(v) => v.iter().map(|c| c.to_string()).collect::<Vec<String>>().join("\n"),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexedString {
  pub data: Vec<char>,
  pub index_map: Vec<Vec<usize>>,
  pub rows: usize,
  pub cols: usize,
}

impl IndexedString {
  
  fn new(input: &str) -> Self {
    let mut data = Vec::new();
    let mut index_map = Vec::new();
    let mut current_row = 0;
    let mut current_col = 0;
    index_map.push(Vec::new());
    for c in input.chars() {
      data.push(c);
      if c == '\n' {
        index_map.push(Vec::new());
        current_row += 1;
        current_col = 0;
      } else {
        index_map[current_row].push(data.len() - 1);
        current_col += 1;
      }
    }
    let rows = index_map.len();
    let cols = if rows > 0 { index_map[0].len() } else { 0 };
    IndexedString {
      data,
      index_map,
      rows,
      cols,
    }
  }

  fn to_string(&self) -> String {
    self.data.iter().collect()
  }

  fn get(&self, row: usize, col: usize) -> Option<char> {
    if row < self.rows {
      let rowz = &self.index_map[row];
      if col < rowz.len() {
        let index = self.index_map[row][col];
        Some(self.data[index])
      } else {
        None
      }
    } else {
      None
    }
  }

  fn set(&mut self, row: usize, col: usize, new_char: char) -> Result<(), String> {
    if row < self.rows {
      let row_indices = &mut self.index_map[row];
      if col < row_indices.len() {
        let index = row_indices[col];
        self.data[index] = new_char;
        Ok(())
      } else {
        Err("Column index out of bounds".to_string())
      }
    } else {
      Err("Row index out of bounds".to_string())
    }
  }
}

// Humanize
// ---------------------------------------------------------------------------

// Turn bytes into something more readable by humans
// Useful for visualizing register dumps, hashes, etc.

pub fn hash_chars(input: &Vec<char>) -> u64 {
  seahash::hash(input.iter().map(|s| String::from(*s)).collect::<String>().as_bytes()) & 0x00FFFFFFFFFFFFFF
}

pub fn hash_bytes(input: &Vec<u8>) -> u64 {
  seahash::hash(input) & 0x00FFFFFFFFFFFFFF
}

pub fn hash_str(input: &str) -> u64 {
  seahash::hash(input.as_bytes()) & 0x00FFFFFFFFFFFFFF
}

pub fn emojify_bytes(bytes: &[u8]) -> String {
  let start = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len() - 1);
  let mut out = String::new();
  for &b in &bytes[start..] {
    out.push_str(EMOJILIST[b as usize]);
  }
  out
}

pub fn humanize_bytes(bytes: &[u8]) -> String {
  let parts: Vec<&str> = bytes
    .iter()
    .enumerate()
    .filter_map(|(ix, &b)| if ix % 2 == 1 { Some(WORDLIST[b as usize]) } else { None })
    .collect();
  parts.join("-")
}

pub fn emojify<T>(num: &T) -> String
where
    T: Display + Copy + TryInto<u128>,
    <T as TryInto<u128>>::Error: std::fmt::Debug,
{
  match (*num).try_into() {
    Ok(v) => {
      let bytes = v.to_be_bytes();
      emojify_bytes(&bytes)
    }
    Err(_) => format!("{}", num),
  }
}

pub fn humanize<T>(num: &T) -> String
where
  T: Display + Copy + TryInto<u128>,
  <T as TryInto<u128>>::Error: Debug,
{
  match (*num).try_into() {
    Ok(v) => {
      let bytes = v.to_be_bytes();
      let first_non_zero = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len() - 1);
      let trimmed = &bytes[first_non_zero..];
      humanize_bytes(trimmed)
    }
    Err(_) => format!("{}", num),
}
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

// Emoji list is for quicker visual scanning/recognition when comparing registers

pub const EMOJILIST: &[&str; 256] = &[
  "🐵","🐶","🐺","🦊","🦝","🐱","🐈","🐈","🦁","🐷","🐮","🦬","🐯","🐴","🫎","🦄","🦓","🦙","🦒","🐘","🦣","🦏","🦛","🐫","🐏","🐭","🐰","🐿️","🦫","🦔","🦇","🐻","🐨","🐼","🦥","🦦","🦨","🦘","🦡","🦃","🐔","🐦","🐧","🕊️","🦅","🦆","🐦‍🔥","🦉","🦤","🦩","🦚","🦜","🐸","🐊","🐢","🦎","🐍","🐲","🦖","🐳","🐬","🦭","🐠","🦈","🐙","🪼","🦀","🦞","🦐","🦑","🐌","🦋","🐛","🐝","🪲","🐞","🦗","🕸️","🪰","🪱","🦠","👻","👽","🐶","🐮","🐚","🪸","🪶","🦧","🪿","🦢","🤖",
  "🌹","🌳","🌴","🌵","🍀","🍁","🍄","🌛","🌞","🪐","⭐","⛅","🌧️","🌨️","🌈","❄️","☃️","☄️","🔥","🌻",
  "🍇","🍉","🍊","🍋","🍋‍🟩","🍌","🍍","🥭","🍎","🍐","🍓","🥝","🍅","🫒","🥥","🥔","🥕","🌽","🌶️","🫑","🥒","🥦","🧄","🧅","🫛","🍦","🍧","🍩","🍪","🍰","🧁","🥧","🍫","🍭","🍞","🥨","🥯","🧇","🍟","🍿","🧃",
  "🎤","🎧","📻","🎷","🪗","🎸","🎹","🎺","🎻","🪇","🥁","⚗️","📷","🧳","🌡️","🧸","🧶","🔎","🕯️","💡","🔦","🔒","🗝️","🪚","🔧","🪛","🔩","⚙️","⚖️","🧰","🧲","🪜","🔬","📡","🧷","🧹","🧺","🪣","🧼","🧽","🧯","🛒",  
  "⏰","🛟","🛩️","🚁","🛰️","🚀","🛸","⚓","🚂","🚑","🚒","🚕","🚗","🚚","🚜","🏎️","🏍️","🛵","🦼","🚲","🛹","🛼","🛞","📰","📦","📫","✏️","🖊️","🖌️","🖍️","📌","📏","✂️","🗑️","🏆","⚾","🏀","🎾","🎳","⛳","⛸️","🤿","🛷","🎯","🪁","🧩","🪅","🎨","🧭","🏔️","🏝️","⛲","⛺","🎠","🛝","🧵","💈","🎪","🛎️","💎","⛵"
];
