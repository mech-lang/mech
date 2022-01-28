#![feature(get_mut_unchecked)]
#![feature(concat_idents)]

extern crate core as rust_core;
extern crate hashbrown;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate num_traits;

#[macro_use]
extern crate lazy_static;
extern crate seahash;
extern crate indexmap;
extern crate bincode;
extern crate num;
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;

mod database;
mod table;
mod transformation;
mod error;
mod core;
mod block;
mod value;
mod function;

pub use self::table::*;
pub use self::database::*;
pub use self::transformation::Transformation;
pub use self::function::*;
pub use self::block::*;
pub use self::core::Core;
pub use self::value::*;
pub use self::error::MechError;

pub type MechString = Vec<char>;

pub type ColumnV<T> = Rc<RefCell<Vec<T>>>;
pub type Reference = Rc<RefCell<Table>>;

#[derive(Clone, Debug)]
pub enum Column {
  F32(ColumnV<f32>),
  F64(ColumnV<f64>),
  U8(ColumnV<u8>),
  U16(ColumnV<u16>),
  U32(ColumnV<u32>),
  U64(ColumnV<u64>),
  U128(ColumnV<u128>),
  Ref(ColumnV<TableId>),
  I8(ColumnV<i8>),
  I16(ColumnV<i16>),
  I32(ColumnV<i32>),
  I64(ColumnV<i64>),
  I128(ColumnV<i128>),
  Index(ColumnV<usize>),
  Bool(ColumnV<bool>),
  String(ColumnV<MechString>),
  Reference((Reference,(ColumnIndex,ColumnIndex))),
  Empty,
}

#[derive(Clone, Debug)]
pub enum ColumnIndex {
  All,
  Index(usize),
  IndexCol(ColumnV<usize>),
  Bool(ColumnV<bool>),
  None,
}

impl Column {

  pub fn copy(&self) -> Column {
    match self {
      Column::U8(col) => Column::U8(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::U16(col) => Column::U16(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::U32(col) => Column::U32(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::U64(col) => Column::U64(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::U128(col) => Column::U128(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::I8(col) => Column::I8(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::I16(col) => Column::I16(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::I32(col) => Column::I32(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::I64(col) => Column::I64(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::I128(col) => Column::I128(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::F32(col) => Column::F32(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::F64(col) => Column::F64(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::Bool(col) => Column::Bool(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::Index(col) => Column::Index(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::String(col) => Column::String(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::Ref(col) => Column::Ref(Rc::new(RefCell::new(col.borrow().clone()))),
      Column::Reference(reference) => Column::Reference(reference.clone()),
      _ => Column::Empty,
    } 
  }

  pub fn get_u8(&self) -> Result<ColumnV<u8>,MechError> {
    match self {
      Column::U8(col) => Ok(col.clone()),
      _ => {return Err(MechError::GenericError(8172));},
    }
  }

  pub fn get_bool(&self) -> Result<ColumnV<bool>,MechError> {
    match self {
      Column::Bool(col) => Ok(col.clone()),
      _ => {return Err(MechError::GenericError(8170));},
    }
  }

  pub fn get_string(&self) -> Result<ColumnV<MechString>,MechError> {
    match self {
      Column::String(col) => Ok(col.clone()),
      _ => {return Err(MechError::GenericError(8171));},
    }
  }

  pub fn get_reference(&self) -> Result<ColumnV<TableId>,MechError> {
    match self {
      Column::Ref(col) => Ok(col.clone()),
      _ => {return Err(MechError::GenericError(8175));},
    }
  }

  pub fn get_u64(&self) -> Result<ColumnV<u64>,MechError> {
    match self {
      Column::U64(col) => Ok(col.clone()),
      _ => {return Err(MechError::GenericError(8173));},
    }
  }

  pub fn to_index(&mut self) -> Result<Column,MechError> {
    match self {
      Column::U64(col) => {
        let mut new_column: Vec<usize> = Vec::new();
        for value in col.borrow().iter() {
          new_column.push(*value as usize);
        }
        Ok(Column::Index(Rc::new(RefCell::new(new_column))))
      }
      _ => Err(MechError::GenericError(8174)),
    }
  }

  pub fn len(&self) -> usize {
    match self {
      Column::U8(col) => col.borrow().len(),
      Column::U16(col) => col.borrow().len(),
      Column::U32(col) => col.borrow().len(),
      Column::U64(col) => col.borrow().len(),
      Column::U128(col) => col.borrow().len(),
      Column::I8(col) => col.borrow().len(),
      Column::I16(col) => col.borrow().len(),
      Column::I32(col) => col.borrow().len(),
      Column::I64(col) => col.borrow().len(),
      Column::I128(col) => col.borrow().len(),
      Column::F32(col) => col.borrow().len(),
      Column::F64(col) => col.borrow().len(),
      Column::Bool(col) => col.borrow().len(),
      Column::String(col) => col.borrow().len(),
      Column::Index(col) => col.borrow().len(),
      Column::Ref(col) => col.borrow().len(),
      Column::Reference((table,index)) => {
        let t = table.borrow();
        t.rows * t.cols
      },
      Column::Empty => 0,
    }
  }

  pub fn logical_len(&self) -> usize {
    match self {
      Column::U8(col) => col.borrow().len(),
      Column::U16(col) => col.borrow().len(),
      Column::U32(col) => col.borrow().len(),
      Column::U64(col) => col.borrow().len(),
      Column::U128(col) => col.borrow().len(),
      Column::I8(col) => col.borrow().len(),
      Column::I16(col) => col.borrow().len(),
      Column::I32(col) => col.borrow().len(),
      Column::I64(col) => col.borrow().len(),
      Column::I128(col) => col.borrow().len(),
      Column::Ref(col) => col.borrow().len(),
      Column::F32(col) => col.borrow().len(),
      Column::F64(col) => col.borrow().len(),
      Column::Bool(col) => col.borrow().iter().fold(0, |acc,x| if *x { acc + 1 } else { acc }),
      Column::String(col) => col.borrow().len(),
      Column::Index(col) => col.borrow().len(),
      Column::Reference((table,index)) => {
        let t = table.borrow();
        t.rows * t.cols
      },
      Column::Empty => 0,
    }    
  }

  pub fn resize(&self, rows: usize) -> Result<(),MechError> {
    match self {
      Column::U8(col) => col.borrow_mut().resize(rows,0),
      Column::U16(col) => col.borrow_mut().resize(rows,0),
      Column::U32(col) => col.borrow_mut().resize(rows,0),
      Column::U64(col) => col.borrow_mut().resize(rows,0),
      Column::U128(col) => col.borrow_mut().resize(rows,0),
      Column::I8(col) => col.borrow_mut().resize(rows,0),
      Column::I16(col) => col.borrow_mut().resize(rows,0),
      Column::I32(col) => col.borrow_mut().resize(rows,0),
      Column::I64(col) => col.borrow_mut().resize(rows,0),
      Column::I128(col) => col.borrow_mut().resize(rows,0),
      Column::F32(col) => col.borrow_mut().resize(rows,0.0),
      Column::F64(col) => col.borrow_mut().resize(rows,0.0),
      Column::Ref(col) => col.borrow_mut().resize(rows,TableId::Local(0)),
      Column::Index(col) => col.borrow_mut().resize(rows,0),
      Column::Bool(col) => col.borrow_mut().resize(rows,false),
      Column::String(col) => col.borrow_mut().resize(rows,vec![]),
      Column::Reference(_) |
      Column::Empty => {return Err(MechError::GenericError(7143));}
    }
    Ok(())
  }

  pub fn kind(&self) -> ValueKind {
    match self {
      Column::F32(_) => ValueKind::F32,
      Column::F64(_) => ValueKind::F64,
      Column::U8(_) => ValueKind::U8,
      Column::U16(_) => ValueKind::U16,
      Column::U32(_) => ValueKind::U32,
      Column::U64(_) => ValueKind::U64,
      Column::U128(_) => ValueKind::U128,
      Column::I8(_) => ValueKind::I8,
      Column::I16(_) => ValueKind::I16,
      Column::I32(_) => ValueKind::I32,
      Column::I64(_) => ValueKind::I64,
      Column::I128(_) => ValueKind::I128,
      Column::Bool(_) => ValueKind::Bool,
      Column::String(_) => ValueKind::String,
      Column::Index(_) => ValueKind::Index,
      Column::Ref(_) => ValueKind::Reference,
      Column::Reference((table,index)) => table.borrow().kind(),
      Column::Empty => ValueKind::Empty,
      _ => ValueKind::Empty,
    }
  }

}

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
  use std::mem::transmute;
  let bytes: [u8; 8] = unsafe { transmute(hash.to_be()) };
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

  
pub struct BoxPrinter {
  pub lines: Vec<LineKind>,
  width: usize,
  drawing: String,
}

#[derive(Debug)]
pub enum LineKind {
  String(String),
  Table(BoxTable),
  Separator,
}

#[derive(Debug)]
pub struct BoxTable {
  pub width: usize,
  pub rows: usize,
  pub cols: usize,
  pub strings: Vec<Vec<String>>,
  pub column_widths: Vec<usize>,
}

impl BoxTable {

  pub fn new(table: &Table) -> BoxTable {
    let mut strings: Vec<Vec<String>> = vec![vec!["".to_string(); table.rows]; table.cols];
    let mut column_widths = vec![0; table.cols];
    for row in 0..table.rows {
      for col in 0..table.cols {
        let value_string = match table.get(row,col) {
          Ok(v) => format!("{:?}", v), 
          _ => format!(""),
        };
        let chars = value_string.chars().collect::<Vec<char>>().len();
        if chars > column_widths[col] {
          column_widths[col] = chars;
        }
        strings[col][row] = value_string;
      }
    }
    BoxTable {
      width: column_widths.iter().sum(),
      rows: table.rows,
      cols: table.cols,
      strings,
      column_widths,
    }
  }

}

impl BoxPrinter {

  pub fn new() -> BoxPrinter {
    BoxPrinter {
      lines: Vec::new(),
      width: 0,
      drawing: "\n┌─┐\n│ │\n└─┘\n".to_string(),
    }
  }

  pub fn add_line(&mut self, lines: String) {
    for line in lines.lines() {
      let chars = line.chars().collect::<Vec<char>>().len();
      if chars > self.width {
        self.width = chars;
      }
      self.lines.push(LineKind::String(line.to_string()));
    }
    self.render_box();
  }

  pub fn add_header(&mut self, text: &str) {
    self.lines.push(LineKind::Separator);
    self.add_line(text.to_string());
    self.lines.push(LineKind::Separator);
  }

  pub fn add_separator(&mut self) {
    self.lines.push(LineKind::Separator);
    self.render_box();
  }

  pub fn add_table(&mut self, table: &Table) {
    let bt = BoxTable::new(table);
    self.width = if bt.width + bt.cols > self.width {
      bt.width + bt.cols - 1
    } else {
      self.width
    };
    self.lines.push(LineKind::Table(bt));
    self.render_box();
  }

  fn render_box(&mut self) {
    let top = "┌".to_string() + &BoxPrinter::format_repeated_char("─", self.width) + &"┐\n".to_string();
    let mut middle = "".to_string();
    let mut bottom = "└".to_string() + &BoxPrinter::format_repeated_char("─", self.width) + &"┘\n".to_string();
    for line in &self.lines {
      match line {
        LineKind::Separator => {
          let boxed_line = "├".to_string() + &BoxPrinter::format_repeated_char("─", self.width) + &"┤\n".to_string();
          middle += &boxed_line;
        }
        LineKind::Table(table) => {
          if table.rows == 0 || table.cols == 0 {
            continue;
          }
          let mut column_widths = table.column_widths.clone();
          if table.width + table.cols < self.width {
            let mut diff = self.width - (table.width + table.cols) + 1;
            let mut ix = 0;
            while diff > 0 {
              let c = column_widths.len();
              column_widths[ix % c] += 1;
              ix += 1;
              diff -= 1; 
            }
          }
          middle += "├"; 
          for col in 0..table.cols-1 {
            middle += &BoxPrinter::format_repeated_char("─", column_widths[col]);
            middle += "┬";
          }
          middle += &BoxPrinter::format_repeated_char("─", *column_widths.last().unwrap());
          middle += "┤\n";
          for row in 0..table.rows {
            let mut boxed_line = "│".to_string();
            for col in 0..table.cols {
              let cell = &table.strings[col][row];
              let chars = cell.chars().collect::<Vec<char>>().len();
              boxed_line += &cell; 
              boxed_line += &BoxPrinter::format_repeated_char(" ", column_widths[col] - chars);
              boxed_line += &"│".to_string();
            }
            boxed_line += &"\n".to_string();
            middle += &boxed_line;
          }
          bottom = "└".to_string(); 
          for col in 0..table.cols-1 {
            bottom += &BoxPrinter::format_repeated_char("─", column_widths[col]);
            bottom += &"┴".to_string();
          }
          bottom += &BoxPrinter::format_repeated_char("─", *column_widths.last().unwrap());
          bottom += &"┘\n".to_string();
        }
        LineKind::String(line) => {
          let chars = line.chars().collect::<Vec<char>>().len();
          if self.width >= chars {
            let boxed_line = "│".to_string() + &line + &BoxPrinter::format_repeated_char(" ", self.width - chars) + &"│\n".to_string();
            middle += &boxed_line;
          } else {
            println!("Line too long: {:?}", line);
          }
        }
      }
    }
    self.drawing = top + &middle + &bottom;
  }

  pub fn print(&self) -> String {
    self.drawing.clone()
  }

  fn format_repeated_char(to_print: &str, n: usize) -> String {
    let mut s = "".to_string();
    for _ in 0..n {
      s = format!("{}{}",s,to_print);
    }
    s
  }

}

impl fmt::Debug for BoxPrinter {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f,"{}",self.drawing)?;
    Ok(())
  }
}

