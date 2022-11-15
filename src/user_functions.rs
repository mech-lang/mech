use crate::*;

use hashbrown::{HashSet, HashMap};

// # User-defined Mech Functions

// These functions are written in Mech rather than Rust. They can use other
// user defined functions or rust functions in the function body. They cannot
// use any machines because thoes are side-effectful, and functions must be
// idempotent. The following statements are also prohibited to be used in a
// user defined function: global table define, table set, table set-update, 
// table append, and all temporal operators. Furthermore, user defined
// functions cannot use global tables at all in the body of the function.
// All inputs that are used must be passed in as an input argument.

#[derive(Clone, Debug)]
pub struct UserFunction {
    pub name: u64,
    pub inputs: HashMap<u64,ValueKind>,
    pub outputs: HashMap<u64,ValueKind>,
    pub transformations: Vec<Transformation>,
}

impl UserFunction {
  pub fn new() -> UserFunction {
    UserFunction {
      name: 0,
      inputs: HashMap::new(),
      outputs: HashMap::new(),
      transformations: Vec::new(),
    }
  }

  pub fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &Out) -> Result<CompiledUserFunction,MechError> {
    let mut fxn_block = Block::new();
    fxn_block.functions = block.functions.clone();
    fxn_block.user_functions = block.user_functions.clone();

    // Resolve input arguments
    for (arg_name, arg_table_id, indices) in arguments {
      match self.inputs.get(arg_name) {
        Some(kind) => {
          let table_ref = block.get_table(arg_table_id)?;
          let block_id = {table_ref.borrow().id.clone()};
          fxn_block.tables.insert_table_ref(table_ref.clone());
          let tfm = Transformation::TableAlias{
            table_id: TableId::Local(block_id), 
            alias: *arg_name,
          };
          fxn_block.add_tfm(tfm);
        },
        _ => (),
      }
    }

    // Compile function steps
    let mut tfms = self.transformations.clone();
    tfms.sort();
    tfms.dedup();
    for tfm in &tfms {
      fxn_block.add_tfm(tfm.clone());
    }

    // Resolve output arguments
    for (name,kind) in self.outputs.iter() {
      let (out_table_id, _, _) = out;
      let out_table_ref = block.get_table(out_table_id)?;
      fxn_block.tables.insert_table_ref(out_table_ref.clone());
      #[cfg(feature = "stdlib")]
      fxn_block.add_tfm(Transformation::Function{
        name: *TABLE_HORIZONTAL__CONCATENATE,
        arguments: vec![(0,TableId::Local(*name),vec![(TableIndex::All,TableIndex::All)])],
        out: (*out_table_id,TableIndex::All,TableIndex::All),
      });
    }
    fxn_block.id = hash_str(&format!("{:?}{:?}{:?}",block.id,self.name,self.inputs));

    let compiled_fxn = CompiledUserFunction{
      name: self.name, 
      inputs: self.inputs.clone(), 
      outputs: self.outputs.clone(), 
      block: fxn_block
    };
    Ok(compiled_fxn)
  }

}

#[derive(Clone, Debug)]
pub struct CompiledUserFunction {
  pub name: u64,
  pub inputs: HashMap<u64,ValueKind>,
  pub outputs: HashMap<u64,ValueKind>,
  pub block: Block,
}

impl MechFunction for CompiledUserFunction {

  fn solve(&self) {
    self.block.solve();
  }
  fn to_string(&self) -> String {
    format!("{:?}", self.block)
  }

}