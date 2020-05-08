#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate mech_core;
extern crate hashbrown;

use hashbrown::HashMap;
use mech_core::{Table, Value, Aliases, Transaction, Interner, TableId, Core, Constraint, Register};

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
  //Core(Core),
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
    let mut column_aliases = Aliases::new(); 
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
  fn call(&self) -> Result<(), String>;
}

#[derive(Copy, Clone)]
pub struct MachineDeclaration {
    pub register: unsafe extern "C" fn(&mut dyn MachineRegistrar),
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