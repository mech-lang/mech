// # Operations

// ## Prelude

use alloc::{String, Vec};

/*
Queries are compiled down to a Plan, which is a sequence of operations that 
map to database operations.

The operations are:

- Scan
- AntiScan
- Filter
- Function
- Project

*/

// ## Functions

#[repr(u8)]
#[derive(Debug, Clone)]
pub enum Function {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
}

struct Plan {
    operations: Vec<Operation>,
}

impl Plan {

    pub fn new() -> Plan {
        Plan {
            operations: Vec::new(),
        }
    }

}

enum Operation {
    Scan,
    Antiscan,
    Filter,
    Function,
    Project,
}


impl Operation {

}