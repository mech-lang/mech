<p align="center">
  <img width="500px" src="https://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like animations, games, and robots. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your project.

[Try](https://mech-lang.org/try/) Mech online in your browser, or follow our progress on our [blog](https://mech-lang.org/blog/).

## Documentation

If this is your first time with Mech, read [Learn Mech in Fifteen Minutes](https://gitlab.com/mech-lang/docs/-/raw/v0.2-beta/III.guides/MechFifteen.mec).

Documentation is hosted online at [mech-lang.org](https://mech-lang.org/docs), and is open sourced on [GitHub](http://github.com/mech-lang/docs).

## Installation

### Binary

You can download the latest release for your platform [here](https://github.com/mech-lang/mech/releases).

### Source

You will need to install [Rust](https://www.rust-lang.org/learn/get-started) on a recent nightly release. Follow these instructions to build the Mech language toolchain, which is packaged in a single executable called "mech":

```bash
git clone https://gitlab.com/mech-lang/mech
cd mech
cargo build --bin mech --release
```

## Project Roadmap

Mech is undergoing a redesign to incorporate state machines into the language.

This work is happening in the v0.2-beta branch of the repository.

The current target for the release of v0.2 is October 2024.

See [ROADMAP.mec](ROADMAP.mec) for more.

## License

Apache 2.0
