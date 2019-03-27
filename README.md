<p align="center">
  <img width="500px" src="http://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like animations, games, and robots. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your problem. 

Read about progress on our [blog](http://mech-lang.org/blog/), follow us on Twitter [@MechLang](https://twitter.com/MechLang), or join the mailing list: [talk@mech-lang.org](http://mech-lang.org/page/community/).

## Program

Organizes Mech cores into a coordinated program. Handles reading files, interfacing with libraries, and persisting changes top disk.

## Contents

- `Program` - holds a Mech core and channels for communicating to a RunLoop.
- `RunLoop` - holds a handle to a thread on which a `ProgramRunner` is running. It also holds channels for communicating between the ProgramRunner and a client, like an editor or a REPL.
- `Persister` - reads from and writes transactions to *.mdb files.
- `ProgramRunner` - Starts an infinite run loop on a thread that continually processes messages received messages.

## Project Status

Mech is currently in the **alpha** stage of development. This means that while some features work and are tested, programs are still likely to crash and produce incorrect results. We've implemented some language features, but many are not yet implemented.

Feel free to use the language for your own satisfaction, but please don't use it for anything important.

## License

Apache 2.0