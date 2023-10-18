use crate::*;

#[cfg(feature = "stdlib")]
use crate::function::table::*;

use hashbrown::{HashSet, HashMap};

// # User-defined Mech Functions


/*

User defined functions are functions written in Mech that can use other user-defined functions or Rust functions in their body, but not machines as they are side-effectful. They are also idempotent and cannot use global tables or temporal operators. Inputs are passed as arguments and outputs must be explicitly defined. A user-defined function is compiled into a block and the input arguments are resolved, the steps are compiled and the output arguments are resolved. The compiled user function can then be used as a Mech function.

In Mech, user-defined functions support both overloading and dynamic dispatch. 

- Overloading refers to defining multiple functions with the same name, but different parameter types or number of parameters. In Mech, the function name and its input arguments' types are used to identify which version of the function to call. This is useful when a function is used in multiple contexts, with different input types or parameter lists.

- Dynamic dispatch refers to the ability to determine at runtime which version of a function to call based on the input types, rather than being determined at compile-time. This is important when working with polymorphic types or when using functions that have different versions for different types.

Mech supports overloading and dynamic dispatch in user-defined functions by allowing the user to specify the input and output types for each function. When a user-defined function is called, Mech uses the input types to determine which version of the function to call. This allows for greater flexibility in the types of inputs that can be used with user-defined functions.
*/


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