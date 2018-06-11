<img width="40%" height="40%" src="https://mechlang.net/img/logo.png">

---

Mech is a language for developing data-driven, reactive systems such as robots and IoT devices. This repository hosts the core of Mech, a small dataflow engine that accepts transactions of changes, and applies them to a compute network. This module is mostly used internally, with user-facing interfaces exposed in the [mech-server](https://gitlab.com/cmontella/mech-server) module.

This module does not rely on the Rust standard library, so it can be compiled and used on bare-bones operating systems.

## Contents

- table - defines a `Table`, the fundamental datatype of Mech. Also defines a `Value`, the 
- database - defines a database of `Tables`. Accepts a `Transaction`, which is a set of `Changes` to the database.
- indexes - defines the various indexes used to quickly look up information in the database.
- runtime - defines `Blocks`, which comprise the compute network.
- operations - defines the primitive operations available to the compute network.

## Usage

```rust
// In your Cargo.toml file, you'll want to include Mech as a dependency:
// mech = {git = "https://gitlab.com/cmontella/mech.git"}
extern crate mech;
// Create a new mech core
let mut core = mech::database::Database::new(change_capacity, table_capcity);

// Create a new table, and add two values to it
let mut txn = Transaction::from_changeset(vec![
  Change::NewTable{tag: 1, rows: 1, columns: 3},
  Change::Add{table: 1, row: 1, column: 1, value: Value::from_u64(1)},
  Change::Add{table: 1, row: 1, column: 2, value: Value::from_u64(2)},
});

// Apply the transaction
core.process_transaction(&txn);

// Table 1:
// ┌───┬───┬───┐
// │ 1 │ 2 │   │
// └───┴───┴───┘

// Create a block that adds two numbers. You can either compile blocks by hand or with
// the mech-syntax compiler.
let mut block = Block::new();
block.add_constraint(Constraint::Scan {table: 1, column: 1, input: 1});
block.add_constraint(Constraint::Scan {table: 1, column: 2, input: 2});
block.add_constraint(Constraint::Identity {source: 1, sink: 1});
  block.add_constraint(Constraint::Identity {source: 2, sink: 2});
block.add_constraint(Constraint::Function {operation: Function::Add, parameters: vec![1, 2], output: 3});
block.add_constraint(Constraint::Insert {output: 3, table: 1, column: 3});
let plan = vec![
  Constraint::Identity {source: 1, sink: 1},
  Constraint::Identity {source: 2, sink: 2},
  Constraint::Function {operation: Function::Add, parameters: vec![1, 2], output: 3},
  Constraint::Insert {output: 3, table: 1, column: 3}
];
block.plan = plan;

// Register the block with the runtime
core.runtime.register_blocks(vec![block]);

// Table 1:
// ┌───┬───┬───┐
// │ 1 │ 2 │ 3 │
// └───┴───┴───┘

// Check that the numbers were added together
assert_eq!(core.store.get_cell(1, 1, 3), Some(Value::from_u64(3)));

// We can add another row to Table 1
let mut txn2 = Transaction::from_changeset(vec![
  Change::Add{table: 1, row: 2, column: 1, value: Value::from_u64(3)},
  Change::Add{table: 1, row: 2, column: 2, value: Value::from_u64(4)},
});
core.process_transaction(&txn2);

// Table 1:
// ┌───┬───┬───┐
// │ 1 │ 2 │ 3 │
// │ 3 │ 4 │ 7 │
// └───┴───┴───┘

assert_eq!(core.store.get_cell(1, 2, 3), Some(Value::from_u64(7)));
```

## License

Apache 2.0