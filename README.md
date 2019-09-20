<p align="center">
  <img width="500px" src="http://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like animations, games, and robots. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your problem. 

You can try Mech online at [try.mech-lang.org](http://try.mech-lang.org).

Usage and installation instructions can be found in the [documentation](http://mech-lang.org/page/learn/) or the [main Mech repository](https://github.com/mech-lang/mech).

Read about progress on our [blog](http://mech-lang.org/blog/), follow us on Twitter [@MechLang](https://twitter.com/MechLang), get live help on our [Gitter channel](https://gitter.im/mech-lang/community), or join the [mailing list](https://groups.google.com/forum/#!forum/mechtalk).


## Welcome

This repository serves as a table of contents for the constellation of tools and utilities that comprise the Mech programming language:

1. [Core](https://gitlab.com/mech-lang/core) - The language runtime. It's a small dataflow engine that accepts transactions of changes, and applies them to a compute network.  
2. [Syntax](https://gitlab.com/mech-lang/syntax) - A compiler for a textual Mech syntax.
3. [Program](https://gitlab.com/mech-lang/program) - Organizes Mech cores into a coordinated program. Handles reading files, interfacing with libraries, and persisting changes top disk.
4. [Server](https://gitlab.com/mech-lang/server) - Provides a server and client for hosting Mech programs and a browser-based editor on a websocket.
5. [Notebook](https://gitlab.com/mech-lang/notebook) / [Mech-wasm](https://github.com/mech-lang/wasm) - A hosted editor for writing Mech applications.

## Documentation

Right now, most Mech features and syntax are undocumented. You can find some minimal documentation [here](http://docs.mech-lang.org), and also the beginning of a tutorial [here](https://github.com/mech-lang/mech/blob/master/examples/tutorial.mec).

Documentation and tooling will be a focus of the next release (v0.0.2).

## Building from Source

You will need to install [Rust](https://www.rust-lang.org/learn/get-started) and [NodeJS](https://nodejs.org/) before building Mech. When those are installed, follow these instructions:

```bash
> git clone https://gitlab.com/mech-lang/mech
> cd mech
> git submodule update --init --recursive
> cd notebook
> npm install
> cd..
> cargo build --bin mech --release
```

## Project Status

Mech is currently in the **v0.0.2 alpha** stage of development. This means that while some features work and are tested, programs are still likely to crash and produce incorrect results. We've implemented some language features, but many are not yet implemented.

Feel free to use the language for your own satisfaction, but please don't use it for anything important.

## License

Apache 2.0