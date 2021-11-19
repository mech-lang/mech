<p align="center">
  <img width="500px" src="http://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like animations, games, and robots. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your project.  

You can try Mech online at [try.mech-lang.org](http://try.mech-lang.org).

Usage and installation instructions can be found in the [documentation](http://mech-lang.org/page/learn/) or the [main Mech repository](https://github.com/mech-lang/mech).

Read about progress on our [blog](http://mech-lang.org/blog/), and follow us on Twitter [@MechLang](https://twitter.com/MechLang).


## Welcome

This repository serves as a table of contents for the constellation of tools and utilities that comprise the Mech programming language:

1. [Core](https://gitlab.com/mech-lang/core) - The language runtime. It's a small dataflow engine that accepts transactions of changes, and applies them to a compute network.  
2. [Syntax](https://gitlab.com/mech-lang/syntax) - A compiler for a textual Mech syntax.
3. [Program](https://gitlab.com/mech-lang/program) - Coordinates distributed Mech cores as a coherent program.

## Documentation

Documentation is hosted online at [mech-lang.org](http://docs.mech-lang.org), and is open sourced on [GitHub](http://github.com/mech-lang/docs).

- Mech platform references
  - Syntax
  - Ecosystem
  - Architecture
- User guides
  - Tutorials
  - How-Tos
  - Mech for X  

## Installation

### From Binary

You can download the the latest release for your platform [here](https://github.com/mech-lang/mech/releases). Or, if you have Rust's Cargo tool installed you can use the following command:

```bash
> cargo install mech
```

### From Source

You will need to install [Rust](https://www.rust-lang.org/learn/get-started) (Mech only works on the "Nightly" release channel) before building Mech. When those are installed, follow these instructions:

```bash
> git clone https://gitlab.com/mech-lang/mech
> cd mech
> cargo build --bin mech --release
```

## Project Roadmap

Mech is currently in the **v0.0.5 alpha** stage of development. This means that while some features work and are tested, programs are still likely to crash and produce incorrect results. We've implemented many language features, but most are incomplete and some are not yet implemented at all. 

The project will hit the **v0.1 beta** stage of development when all currently-planned features have been implemented at least at as a prototype.

See [ROADMAP.md](ROADMAP.md) for more.

## License

Apache 2.0
