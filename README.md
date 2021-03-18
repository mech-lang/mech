<p align="center">
  <img width="400px" src="http://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like animations, games, and robots. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your work. 

You can try Mech online at [try.mech-lang.org](http://try.mech-lang.org).

Usage and installation instructions can be found in the [documentation](http://mech-lang.org/page/learn/) or the [main Mech repository](https://github.com/mech-lang/mech).

Read about progress on our [blog](http://mech-lang.org/blog/) and follow us on Twitter [@MechLang](https://twitter.com/MechLang).

# Core

The language runtime. It's a small dataflow engine that accepts transactions of changes, and applies them to a compute network.  

## Contents

- **value** - defines a `Value`, a unified datatype for Mech. A value can be empty, a boolean, a string, a reference to a table, a number literal, or a quantity (number + unit).
- **table** - defines a `Table`, the core data structure of Mech. A table is a 2D array of values.
- **block** - defines a `Block`, which is the ubiquitous unit of code in Mech. A block is comprised of transformations on input tables. These transformations can either modify existing tables or create new tables.
- **database** - defines a `Database` of tables. Databases accept `Transactions`, which are sets of `Changes` to the database.
- **runtime** - defines a `Runtime`, which orchestrate the execution of blocks that comprise the compute graph.
- **operations** - defines the primitive operations that can be performed by nodes in the compute network. These include basic mathematical, comparison, and logic operations that can be performed on values.
- **errors** - defines an `Error`, which holds the information necessary to track and render error messages.
- **core** - defines a `Core`, which wraps all the other modules into a struct with user-facing interfaces. Also defines a standard library of functions that can be loaded at runtime.

##  Status

Mech is currently **alpha**. This means that while some features work and are tested, programs are still likely to crash and produce incorrect results. We've implemented some language features, but many are not yet implemented.

Feel free to use the language for your own satisfaction, but please don't use it for anything important.

## License

Apache 2.0
