<p align="center">
  <img width="400px" src="https://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like robots, games, and animations. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your project. 

You can try Mech online at [https://mech-lang.org/try](https://mech-lang.org/try).

Usage and installation instructions can be found in the [documentation](https://mech-lang.org/#/docs/index.mec) or the [main Mech repository](https://github.com/mech-lang/mech).

Be sure to follow our [blog](https://mech-lang.org/blog/)([RSS](https://mech-lang.org/feed.xml))!

## Program

Organizes Mech cores into a coordinated program. Handles reading files, interfacing with libraries, and persisting changes top disk.

## Contents

- **program** - holds a Mech core and channels for communicating to a RunLoop.
- **runloop** - holds a handle to a thread on which a Mech program is running. It also holds channels for communicating between and editor, REPL, or remote core.
- **persister** - reads from and writes transactions to *.blx files.

## Project Status

Mech is currently in the **beta** stage of development. This means that the language is at a suitable stage for a wider audience. While most language feature implementations are started, none are finished, and some new features may, while others could be removed. Backwards and forwards compatibility of Mech programs is not guaranteed at this time. 

## License

Apache 2.0