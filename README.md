<p align="center">
  <img width="400px" src="https://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like robots, games, and animations. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your project. 

You can try Mech online at [https://mech-lang.org/try](https://mech-lang.org/try).

Usage and installation instructions can be found in the [documentation](https://mech-lang.org/#/docs/index.mec) or the [main Mech repository](https://github.com/mech-lang/mech).

Be sure to follow our [blog](https://mech-lang.org/blog/)([RSS](https://mech-lang.org/feed.xml))!

# Core

The language runtime. It's a small dataflow engine that accepts transactions of changes, and applies them to a compute network.  

## Contents

- **block** - defines a `Block`, which is the ubiquitous unit of code in Mech. A block is comprised of transformations on input tables. These transformations can either modify existing tables or create new tables.
- **table** - defines a `Table`, the core data structure of Mech. A table is a 2D array of values.
- **column** - defines a `Column`, which is a vector of values.
- **value** - defines a `Value`, a unified datatype for Mech. A value can be empty, a boolean, a string, a reference to another table, a number literal.
- **database** - defines a `Database` of tables. Databases accept `Transactions`, which are sets of `Changes` to the database.
- **function** - defines the standard library for Mech, including basic indexing, mathematical, comparison, and logic functions.
- **error** - defines an `MechError`, which holds the information necessary to track and render error messages.

## Project Status

Mech is currently in the **beta** stage of development. This means that the language is at a suitable stage for a wider audience. While most language feature implementations are started, none are finished, and some new features may, while others could be removed. Backwards and forwards compatibility of Mech programs is not guaranteed at this time. 

## License

Apache 2.0
