<p align="center">
  <img width="500px" src="https://mech-lang.org/img/logo.png" alt="Mech Logo">
</p>

**Mech** is a language for building **data-driven** and **reactive** systems like robots, games, user interfaces, and more. It simplifies **composing**, **transforming**, and **distributing** data, so you can focus on the core complexities of your project.

[Try Mech](https://mech-lang.org/try/) online in your browser or stay updated through our [blog](https://mech-lang.org/blog/).

## 📚 Documentation

New to Mech? Start with [Learn Mech in Fifteen Minutes](https://gitlab.com/mech-lang/docs/-/raw/v0.2-beta/III.guides/MechFifteen.mec).

Comprehensive documentation is available at [mech-lang.org](https://mech-lang.org/docs) and open-sourced on [GitHub](http://github.com/mech-lang/docs).

## 📂 Download and Install

### 💾 From Binary

Download the latest release for your platform [here](https://github.com/mech-lang/mech/releases/latest).

### 📦 From Source

To build Mech from source, you’ll first need to install [Rust](https://www.rust-lang.org/learn/get-started) (make sure to install a recent version on the nightly release channel). 

Then Follow the instructions below to compile the Mech toolchain, bundled in a single executable called `mech`:

```bash
git clone https://gitlab.com/mech-lang/mech
cd mech
cargo build --bin mech --release
```

### 🪐 Community

- 👾 [Discord](https://discord.gg/asqP25NNTH) - for live chat
- 🐙 [GitHub](https://github.com/mech-lang) - for code and issues
- 📺 [YouTube](https://youtube.com/@coreymontella3520?si=EUy2Mrv1aNo-4uQr) - for video tutorials
- 🛸 [Reddit](https://www.reddit.com/r/mechlang/) - for help and discussion

## 🗺️ Project Roadmap

Mech v0.2 is currecntly **beta** status, meaning most intended features are implemented, but there are still rough edges and a general lack of documentation.

This release focuses on specifying data and formulas.

A Brief Roadmap:

- 📍 v0.2 - data specification - formulas, defining and manipulating data
- v0.3 - program specification - functions, modules, state machines
- v0.4 - system specification - tools, distributed programs, capabilities

For more details, read the [ROADMAP](ROADMAP.mec).

A new version of Mech is [released every week](https://github.com/mech-lang/mech/releases).

## 🐲 Notice

Mech should be considered unstable and therefore unfit for use in critical systems until v1.0 is released.

## ⚖️ License

Licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0).