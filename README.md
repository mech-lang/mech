<p align="center">
  <img width="500px" src="http://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like animations, games, and robots. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your problem. 

You can try Mech online at [try.mech-lang.org](https://try.mech-lang.org).

Usage and installation instructions can be found in the [documentation](http://mech-lang.org/page/learn/) or the [main Mech repository](https://github.com/mech-lang/mech).

Read about progress on our [blog](http://mech-lang.org/blog/), follow us on Twitter [@MechLang](https://twitter.com/MechLang), get live help on our [Gitter channel](https://gitter.im/mech-lang/community), or join the [mailing list](https://groups.google.com/forum/#!forum/mechtalk).

# Core

The language runtime. It's a small dataflow engine that accepts transactions of changes, and applies them to a compute network.  

## Contents

- **table** - defines a `Table`, the core data structure of Mech. Also defines a `Value`, which unifies the various data types (Number, String, Bool, Reference).
- **database** - defines a `Database` of tables. Databases accept `Transactions`, which is are sets of `Changes` to the database.
- **runtime** - defines a `Runtime`, which orchestrates the compute graph; and `Blocks`, which comprise the compute graph.
- **indexes** - defines the various indexes used to quickly look up information in the database
- **operations** - defines the primitive operations that can be performed by nodes in the compute network.
- **errors** - defines an `Error`, which holds the information necessary to track and render error messages.

## Project Status

Mech is currently in the **alpha** stage of development. This means that while some features work and are tested, programs are still likely to crash and produce incorrect results. We've implemented some language features, but many are not yet implemented.

Feel free to use the language for your own satisfaction, but please don't use it for anything important.

## License

Apache 2.0