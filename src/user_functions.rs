use crate::*;

use hashbrown::HashSet;

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
    pub inputs: HashSet<(TableId,ValueKind)>,
    pub outputs: HashSet<(TableId,ValueKind)>,
    pub transformations: Vec<Transformation>,
    pub plan: Plan,
}

impl UserFunction {
    pub fn new() -> UserFunction {
      UserFunction {
        name: 0,
        inputs: HashSet::new(),
        outputs: HashSet::new(),
        transformations: Vec::new(),
        plan: Plan::new(),
      }
    }

}