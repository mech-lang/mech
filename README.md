<img width="40%" height="40%" src="https://mechlang.net/img/logo.png">

Mech is a language for developing **data-driven**, **reactive** systems like animations, games, and robots. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your problem. 

Read about progress on our [blog](https://mechlang.net/blog/), follow us on Twitter [@MechLang](https://twitter.com/MechLang), or join the mailing list: [talk@mechlang.net](https://mechlang.net/page/community/).

## Welcome

This repository serves as a sort of table of contents for the constellation of tools and utilities that comprise the Mech programming language:

1. [Core](https://gitlab.com/mech-lang/core) - The language runtime. It's a small dataflow engine that accepts transactions of changes, and applies them to a compute network.  
2. [Syntax](https://gitlab.com/mech-lang/syntax) - A compiler for a textual Mech syntax.
3. [Server](https://gitlab.com/mech-lang/server) - A server that hosts Mech for any websocket client
4. [Notebook](https://gitlab.com/mech-lang/notebook) - A browser-based interface that connects to a Mech server. Meant for performing calculations and drawing graphs.

## Building from Source

You will need to install Rust before building Mech.

```bash
> git clone https://gitlab.com/mech-lang/mech
> cd mech
> git submodule update --init --recursive
> cargo build --bin mech --release
```

## License

Apache 2.0