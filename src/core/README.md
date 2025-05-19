<p align="center">
  <img width="500px" src="https://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like animations, games, and robots. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your project.

[Try](https://try.mech-lang.org) Mech online in your browser, or follow our progress on our [blog](https://mech-lang.org/blog/).

# Core

The language runtime. It's a small dataflow engine that accepts transactions of changes, and applies them to a compute network.  

## Contents

- **interpreter** - The Mech interpreter, which executes Mech bytecode.
- **value** - Defines `Value`, a unified datatype
- **kind** - Defines `Kind`, which is used to annotate the kind of each varible
- **error** - Define `MechError`, an error type that is used throughout the Mech system.
- **functions** - User defined functions
- **matrix** - Mech `Matrix` wraps NDArray for fast matrix computations
- **nodes** - Defines various nodes which comprise the Mech AST.
- **types** - Defines various types used by the Rust implementation of the Mech compiler.

## Project Status

Mech is currently in the **beta** stage of development. This means that the language is at a suitable stage for a wider audience. While most language feature implementations are started, none are finished, and some new features may, while others could be removed. Backwards and forwards compatibility of Mech programs is not guaranteed at this time. 

## License

Apache 2.0
