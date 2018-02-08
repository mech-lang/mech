// # Operations

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

    pub fn make_scan() -> Operation::Scan {

    }




}