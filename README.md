<p align="center">
  <img width="500px" src="http://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like animations, games, and robots. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your project.

You can try Mech online at [try.mech-lang.org](http://try.mech-lang.org).

Usage and installation instructions can be found in the [documentation](http://mech-lang.org/page/learn/) or the [main Mech repository](https://github.com/mech-lang/mech).

Read about progress on our [blog](http://mech-lang.org/blog/), and follow us on Twitter [@MechLang](https://twitter.com/MechLang).

# Mech Syntax

A toolchain for compiling textual syntax into Mech blocks.

## Contents

- **lexer** - defines a `Token`, which represents a character class.
- **parser** - converts text into a parse tree, with tokens as leaves.
- **compiler** - converts a parse tree to a syntax tree. Also handles converting a syntax tree to block constraints.
- **formatter** - converts a parse tree into formatted text.

This branch also contains various editor modes that enable syntax highlighting in popular IDEs. Modes are available for:

- VS Code

## Project Status

Mech is currently in the **alpha** stage of development. This means that while some features work and are tested, programs are still likely to crash and produce incorrect results. We've implemented some language features, but many are not yet implemented.

Feel free to use the language for your own satisfaction, but please don't use it for anything important.

## License

Apache 2.0