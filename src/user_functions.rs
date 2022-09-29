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
    pub plan: Plan,
}

impl UserFunction {
    pub fn new() -> UserFunction {
      UserFunction {
        name: 0,
        inputs: HashMap::new(),
        outputs: HashMap::new(),
        transformations: Vec::new(),
        plan: Plan::new(),
      }
    }

    pub fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &Out) -> Result<(),MechError> {
      //println!("{:?}", block);
      let mut input_refs = HashMap::new();

      for (arg_name, arg_table_id, indices) in arguments {
        match self.inputs.get(arg_name) {
          Some(kind) => {
            let table_ref = block.get_table(arg_table_id)?;
            input_refs.insert(*arg_name,table_ref.clone());
          },
          _ => (),
        }
      }
    

      println!("{:?}", input_refs);

      Ok(())
    }

}