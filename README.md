<p align="center">
  <img width="400px" src="https://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like robots, games, and animations. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your project. 

You can try Mech online at [https://mech-lang.org/try](https://mech-lang.org/try).

Usage and installation instructions can be found in the [documentation](https://mech-lang.org/#/docs/index.mec) or the [main Mech repository](https://github.com/mech-lang/mech).

Be sure to follow our [blog](https://mech-lang.org/blog/)([RSS](https://mech-lang.org/feed.xml))!

# Syntax

A toolchain for compiling textual Mech source code.

## Contents

- **parser** - converts text into a parse tree.
- **ast** - converts parse tree into abstract syntax tree.
- **compiler** - converts abstract syntax tree into blocks.
- **formatter** - converts a parse tree into text.

This branch also contains various editor modes that implement the language server protocol. Modes are available for:

- [VS Code](https://marketplace.visualstudio.com/items?itemName=Mech.Mech)
- EMACS (Coming soon...)
- VIM (Coming soon...)

##  Project Status

Mech is currently in the **beta** stage of development. This means that the language is at a suitable stage for a wider audience. While most language feature implementations are started, none are finished, and some new features may, while others could be removed. Backwards and forwards compatibility of Mech programs is not guaranteed at this time. 

## License

Apache 2.0