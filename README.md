<img width="40%" height="40%" src="https://mechlang.net/img/logo.png">

---

Mech is a language for developing data-driven, reactive systems like animations, games, and robots. It makes composing, transforming, and distributing data easy, allowing you to focus on the essential complexity of your problem.

There are three components to Mech:

1. Core (this repository) - A small dataflow engine that accepts transactions of changes, and applies them to a compute network.  
2. [Server](https://gitlab.com/cmontella/mech-server) - Hosts Mech cores for connected clients. 
3. [Notebook](https://gitlab.com/cmontella/mech-notebook) - A graphical interface that connects to a Mech server.

Mech core does not rely on the Rust standard library, so it can be compiled and used on bare-bones operating systems (check out [HiveMind OS](https://gitlab.com/cmontella/hvemind) for an example of this).

## Contents

- table - defines a `Table`, the core data structure of Mech. Also defines a `Value`, which unifies the various data types (Number, String, Bool, Table).
- database - defines a database of `Tables`. Accepts a `Transaction`, which is a set of `Changes` to the database.
- indexes - defines the various indexes used to quickly look up information in the database.
- runtime - defines `Blocks`, which comprise the compute network.
- operations - defines the primitive operations available to the compute network.

## Usage

You can use Mech core in your Rust project:

```rust
// In Cargo.toml, include Mech as a dependency:
// mech = {git = "https://gitlab.com/cmontella/mech.git"}
extern crate mech;
use mech::{Core, Transaction, Block, Value};

// Create a new mech core
let mut core = Core::new(change_capacity, table_capacity);

// Create a new table, and add two values to it
let mut txn = Transaction::from_text("#add += [5 3]");

// Apply the transaction
core.process_transaction(&txn);

// #add:
// ┌───┬───┬───┐
// │ 5 │ 3 │   │
// └───┴───┴───┘

// Create a block that adds two numbers.
let mut block = Block::new("#add[3] = #add[1] + #add[2]");

// Register the block with the runtime
core.runtime.register_blocks(vec![block]);

// #add:
// ┌───┬───┬───┐
// │ 5 │ 3 │ 8 │
// └───┴───┴───┘

// Check that the numbers were added together
assert_eq!(core.get_cell("add", 1, 3), Some(Value::from_u64(8)));

// We can add another row to Table 1
let mut txn2 = Transaction::from_text("#add += [3 4]");
core.process_transaction(&txn2);

// #add:
// ┌───┬───┬───┐
// │ 5 │ 3 │ 8 │
// │ 3 │ 4 │ 7 │
// └───┴───┴───┘

// Notice the second row was automatically added
assert_eq!(core.get_cell("add", 2, 3), Some(Value::from_u64(7)));
```

## License

Apache 2.0