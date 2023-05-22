<p align="center">
  <img width="500px" src="https://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like animations, games, and robots. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your project.

You can try Mech online at [try.mech-lang.org](http://try.mech-lang.org).

Usage and installation instructions can be found in the [documentation](https://mech-lang.org/page/learn/) or the [main Mech repository](https://github.com/mech-lang/mech).

Read about progress on our [blog](https://mech-lang.org/blog/), and follow us on Twitter [@MechLang](https://twitter.com/MechLang).

## Documentation

Right now, most Mech features and syntax are undocumented. You can find some minimal documentation [here](http://docs.mech-lang.org), and also the beginning of a tutorial [here](http://docs.mech-lang.org/#/docs/tutorial.mec).

## Build Instructions

To build the Mech Notebook, you'll need [wasm-pack](https://github.com/rustwasm/wasm-pack), which requires Rust 1.30.0 or later.

```bash
wasm-pack build --target web
```

## Project Status

Mech is currently in the **v0.0.5 alpha** stage of development. This means that while some features work and are tested, programs are still likely to crash and produce incorrect results. We've implemented some language features, but many are not yet implemented.

Feel free to use the language for your own satisfaction, but please don't use it for anything important.

## License

Apache 2.0