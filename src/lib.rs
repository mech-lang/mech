#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate mech_core;
extern crate hashbrown;
extern crate crossbeam_channel;

use hashbrown::HashMap;
use mech_core::{Table, Value, Transaction, TableId, Constraint, Register, Change};

use crossbeam_channel::Sender;

// ## Client Message

#[derive(Serialize, Deserialize, Debug)]
pub enum WebsocketMessage {
  Listening(Register),
  Control(u8),
  Code(MechCode),
  Table(NetworkTable),
  RemoveBlock(usize),
  Transaction(Transaction),
}

// Run loop messages are sent to the run loop from the client

#[derive(Debug, Clone)]
pub enum RunLoopMessage {
  Stop,
  StepBack,
  StepForward,
  Pause,
  Resume,
  Clear,
  PrintCore(Option<u64>),
  PrintRuntime,
  Listening(Register),
  Table(NetworkTable),
  GetTable(u64),
  Transaction(Transaction),
  Code((u64,MechCode)),
  EchoCode(String),
  Blocks(Vec<MiniBlock>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiniBlock {
  pub constraints: Vec<(String, Vec<Constraint>)>,
}

impl MiniBlock {
  
  pub fn new() -> MiniBlock { 
    MiniBlock {
      constraints: Vec::with_capacity(1),
    }
  }

}

// ## Value

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Value {
  Number(Quantity),
  //Bool(bool),
  //Reference(TableId),
  //Empty,
}

impl Value {

  pub fn from_string(string: String) -> Value {
    Value::Number(0)
  }

  pub fn from_str(string: &str) -> Value {
    Value::Number(0)
  }

  pub fn from_bool(boolean: bool) -> Value {
    Value::Number(0)
  }

  pub fn from_u64(num: u64) -> Value {
    Value::Number(num.to_quantity())
  }

  pub fn from_quantity(num: Quantity) -> Value {
    Value::Number(num)
  }

  pub fn from_i64(num: i64) -> Value {
    Value::Number(num.to_quantity())
  }

  pub fn from_f64(num: f64) -> Value {
    Value::Number(num.to_quantity())
  }

  pub fn as_quantity(&self) -> Option<Quantity> {
    match self {
      Value::Number(n) => Some(*n),
      //Value::Empty => Some(0.to_quantity()),
      _ => None,
    }
  }

  pub fn as_u64(&self) -> Option<u64> {
    match self {
      Value::Number(n) => Some(n.to_float() as u64),
      //Value::Reference(TableId::Local(n)) => Some(*n),
      //Value::Reference(TableId::Global(n)) => Some(*n),
      _ => None,
    }
  }

  pub fn as_float(&self) -> Option<f64> {
    match self {
      Value::Number(n) => Some(n.to_float()),
      _ => None,
    }
  }

  pub fn as_i64(&self) -> Option<i64> {
    match self {
      Value::Number(n) => Some(n.mantissa()),
      _ => None,
    }
  }

  pub fn as_string(&self) -> Option<String> {
    match self {
      //Value::String(n) => Some(n.clone()),
      Value::Number(q) => Some(q.format()),
      //Value::Reference(TableId::Global(r)) |
      //Value::Reference(TableId::Local(r)) => {
      //  Some(format!("{:?}", r))
      //},
      //Value::Empty => Some(String::from("")),
      //Value::Bool(t) => match t {
      //  true => Some(String::from("true")),
      //  false => Some(String::from("false")),
      //},
      _ => None,
    }
  }

  /*pub fn equal(&self, other: &Value) -> Option<bool> {
    match (self, other) {
      (Value::String(ref x), Value::String(ref y)) => {
        Some(x.to_owned() == y.to_owned())
      }
      _ => None,
    }
  }*/

  /*pub fn not_equal(&self, other: &Value) -> Option<bool> {
    match (self, other) {
      (Value::String(ref x), Value::String(ref y)) => {
        Some(x.to_owned() != y.to_owned())
      }
      _ => None,
    }
  }*/

  pub fn less_than(&self, other: &Value) -> Option<bool> {
    None
  }

  pub fn less_than_equal(&self, other: &Value) -> Option<bool> {
    None
  }

  pub fn greater_than(&self, other: &Value) -> Option<bool> {
    None
  }

  pub fn greater_than_equal(&self, other: &Value) -> Option<bool> {
    None
  }

  pub fn add(&self, other: &Value) -> Option<bool> {
    None
  }

  pub fn sub(&self, other: &Value) -> Option<bool> {
    None
  }

  pub fn multiply(&self, other: &Value) -> Option<bool> {
    None
  }

  pub fn divide(&self, other: &Value) -> Option<bool> {
    None
  }

}

impl fmt::Debug for Value {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &Value::Number(x) => write!(f, "{}", x.to_string()),
      //&Value::String(ref x) => write!(f, "{}", x),
      //&Value::Empty => write!(f, ""),
      //&Value::Bool(ref b) => write!(f, "{}", b),
      //&Value::Reference(ref b) => write!(f, "{:?}", b),
    }
  }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MechCode {
  String(String),
  MiniBlocks(Vec<MiniBlock>),
}

// TODO This is a kludge to get around having to write a serialize method for
// hashmaps.... fix this!
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkTable {
  pub id: u64,
  pub rows: u64,
  pub columns: u64,
  pub column_aliases: Vec<(u64, u64)>,
  pub column_index_to_alias: Vec<Option<u64>>, 
  pub row_aliases: Vec<(u64, u64)>,
  pub data: Vec<Vec<Value>>,
}

impl NetworkTable {
  pub fn new(table: &Table) -> NetworkTable {
    let mut column_aliases: Vec<(u64,u64)> = Vec::new(); 
    for (k,v) in table.column_aliases.iter() {
      column_aliases.push((k.clone(),v.clone()));
    };
    let mut row_aliases: Vec<(u64,u64)> = Vec::new(); 
    for (k,v) in table.row_aliases.iter() {
      row_aliases.push((k.clone(),v.clone()));
    };
    NetworkTable {
      id: table.id.clone(),
      rows: table.rows.clone(),
      columns: table.columns.clone(),
      column_aliases,
      column_index_to_alias: table.column_index_to_alias.clone(),
      row_aliases,
      data: table.data.clone(),
    }
  }

  pub fn to_table(&mut self) -> Table {
    let mut column_aliases = HashMap::new(); 
    for (k,v) in self.column_aliases.iter() {
      column_aliases.insert(k.clone(),v.clone());
    };
    let mut row_aliases: HashMap<u64,u64> = HashMap::new(); 
    for (k,v) in self.row_aliases.iter() {
      row_aliases.insert(k.clone(),v.clone());
    };
    Table {
      id: self.id.clone(),
      rows: self.rows.clone(),
      columns: self.columns.clone(),
      column_aliases,
      column_index_to_alias: self.column_index_to_alias.clone(),
      row_aliases: row_aliases,
      data: self.data.clone(),
    }
  }


}

pub trait Machine {
  fn name(&self) -> String;
  fn id(&self) -> u64;
  fn on_change(&self, change: &Change) -> Result<(), String>;
}

#[derive(Copy, Clone)]
pub struct MachineDeclaration {
    pub register: unsafe extern "C" fn(&mut dyn MachineRegistrar, outgoing: Sender<RunLoopMessage>)->Vec<Change>,
}

pub trait MachineRegistrar {
    fn register_machine(&mut self, machine: Box<dyn Machine>);
}

#[macro_export]
macro_rules! export_machine {
    ($name:ident, $register:expr) => {
        #[doc(hidden)]
        #[no_mangle]
        pub static $name: $crate::MachineDeclaration =
            $crate::MachineDeclaration {
                register: $register,
            };
    };
}

