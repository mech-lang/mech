<p align="center">
  <img width="500px" src="https://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like animations, games, and robots. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your project.

You can try Mech online at [try.mech-lang.org](http://try.mech-lang.org).

Usage and installation instructions can be found in the [documentation](https://mech-lang.org/page/learn/) or the [main Mech repository](https://github.com/mech-lang/mech).

Read about progress on our [blog](https://mech-lang.org/blog/), and follow us on Twitter [@MechLang](https://twitter.com/MechLang).


## Documentation

Documentation is hosted online at [mech-lang.org](http://docs.mech-lang.org), and is open sourced on [GitHub](http://github.com/mech-lang/docs).


## Installation

### From Binary

You can download the latest release for your platform [here](https://github.com/mech-lang/mech/releases).

### From Source

You will need to install [Rust](https://www.rust-lang.org/learn/get-started) on a recent nightly release, and [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/). When those are installed, follow these instructions:

```bash
git clone https://gitlab.com/mech-lang/mech
cd mech
wasm-pack build wasm-notebook --target web
cargo build --bin mech --release
```

## Project Roadmap

Mech is undergoing a redesign to incorporate state machines into the language.

This work is happening in the v0.2-beta branch of the repository.

The current target for the release of v0.2 is October 2024.

See [ROADMAP.md](ROADMAP.md) for more.

## License

Apache 2.0
