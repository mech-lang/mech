<p align="center">
  <img width="500px" src="https://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like animations, games, and robots. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your problem. 

You can try Mech online at [try.mech-lang.org](https://try.mech-lang.org).

Usage and installation instructions can be found in the [documentation](https://mech-lang.org/page/learn/) or the [main Mech repository](https://github.com/mech-lang/mech).

Read about progress on our [blog](https://mech-lang.org/blog/), follow us on Twitter [@MechLang](https://twitter.com/MechLang), get live help on our [Gitter channel](https://gitter.im/mech-lang/community), or join the [mailing list](https://groups.google.com/forum/#!forum/mechtalk).


## Welcome

This repository serves as a table of contents for the constellation of tools and utilities that comprise the Mech programming language:

1. [Core](https://gitlab.com/mech-lang/core) - The language runtime. It's a small dataflow engine that accepts transactions of changes, and applies them to a compute network.  
2. [Syntax](https://gitlab.com/mech-lang/syntax) - A compiler for a textual Mech syntax.
3. [Server](https://gitlab.com/mech-lang/server) - A server that hosts Mech for any websocket client
4. [Notebook](https://gitlab.com/mech-lang/notebook) - A browser-based interface that connects to a Mech server. Meant for performing calculations and drawing graphs.

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

Mech is currently in the **alpha** stage of development. This means that while some features work and are tested, programs are still likely to crash and produce incorrect results. We've implemented some language features, but many are not yet implemented.

Feel free to use the language for your own satisfaction, but please don't use it for anything important.

## License

Apache 2.0