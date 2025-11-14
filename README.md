<p align="center">
  <img width="500px" src="https://mech-lang.org/img/logo.png" alt="Mech Logo">
</p>

**Mech is for building** data-driven, reactive systems like **robots**, games, embedded devices and more.

**It simplifies data** distribution, transformation, and analysis **so you can focus** on your project.

- [Try Mech](https://try.mech-lang.org) online in your browser. 
- [Read the docs](https://docs.mech-lang.org) to learn the language.
- [Follow our blog](https://mech-lang.org/blog/) to stay updated on new developments.

The following code implements FizzBuzz in Mech:

<p align="center">
  <img width="680px" src="https://mech-lang.org/img/fizzbuzzblock.png" alt='The classic FizzBuzz program expressed in Mech: x := 1..=10;~out<[string]>:=x;ix2:=(x % 2) == 0;ix3:=(x % 3) == 0;out[ix2]="✨";out[ix3]="🐝";out[ix2 && ix3]="✨🐝";'>
</p>

See the docs for an [extended version](https://docs.mech-lang.org/examples/fizzbuzz.html) that runs live in your browser.

Some notable features of Mech are demonstrated in this short program:

- **Concise Syntax**: Expressive and flexible, with no need for keywords or semicolons; Mech programs are generally shorter than in other languages.
- **Broadcast Operations**: Vector operations apply elementwise automatically, removing explicit loops and enabling efficient vectorization.
- **Logical Indexing**: Vector elements are conditionally selected using broadcast logic operations, enabling declarative iteration and parallel execution.
- **Type Inference**: Variable and expression types are inferred, so explicit type declarations are often unnecessary.
- **Immutable**: Variables are immutable unless specified, promoting safer code and easier reasoning.
- **Rich Text**: Mech programs support rich formatting and literate programming through Mechdown, a Markdown dialect.

## 💾 Download and Install

### From Binary

[Download](https://docs.mech-lang.org/getting-started/install.html) the precompiled latest release for your platform.

### From Source

To build Mech from source, you’ll first need to install [Rust](https://www.rust-lang.org/learn/get-started) (make sure to install a recent version on the nightly release channel, currently we are developing against `nightly-2025-07-31`). 

Then follow the instructions below to compile the Mech toolchain, bundled in a single executable called `mech`:

```bash
git clone https://gitlab.com/mech-lang/mech
cd mech
cargo build --bin mech --release
```

Alternatively, you can install Mech directly via Rust's [Cargo](https://crates.io/crates/mech) utility:

```bash
cargo install mech
```

## 📚 Documentation

New to Mech? Start with [Learn Mech in Fifteen Minutes](https://docs.mech-lang.org/guides/mech-in-fifteen-minutes.html).

Comprehensive documentation is available at [docs.mech-lang.org](https://docs.mech-lang.org) and open-sourced on [GitHub](https://github.com/mech-lang/mech/tree/main/docs).

## 🪐 Community

The Mech community stays active at a few places around the Internet:

- [Discord](https://discord.gg/asqP25NNTH) - for live chat
- [GitHub](https://github.com/mech-lang) - for code and issues
- [YouTube](https://www.youtube.com/@MechLang) - for video tutorials
- [Reddit](https://www.reddit.com/r/mechlang/) - for help and general discussion
- [Mailing List](https://groups.google.com/g/mechtalk) - for dev discussion

Feel free to stop by and introduce yourself -- we're happy to meet new users and answer questions! 

## 🗺️ Project Roadmap

Mech v0.2 is currently **beta** status, meaning most intended features are implemented, but rough edges abound and there is a general lack of documentation. Development is focused on testing and documentation.

A Brief Roadmap:

- ☑️ [v0.1](https://github.com/mech-lang/mech/tree/v0.1-beta) - proof of concept system - minimum viable language implementation
- 📍 [v0.2](https://github.com/mech-lang/mech/tree/v0.2-beta) - data specification - formulas, defining and manipulating data
- ☐ v0.3 - program specification - functions, modules, state machines
- ☐ v0.4 - system specification - tools, distributed programs, capabilities

For more details, read the [ROADMAP](https://docs.mech-lang.org/design/ROADMAP.html).

A new version of Mech is [released every week](https://github.com/mech-lang/mech/releases).

## 🐲 Notice

Mech should be considered unstable and therefore unfit for use in critical systems until v1.0 is released.

## ⚖️ License

Licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0).